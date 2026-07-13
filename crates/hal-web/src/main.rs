//! A minimal, framework-free WebAssembly front-end for `hal-sdk`.
//!
//! The whole application is written in Rust: it builds the DOM with `web-sys`
//! and calls the SDK directly from the browser (the HAL API allows
//! cross-origin requests, so no backend is required). Build it with `trunk`.
//!
//! Features: quick (debounced) search, fine-grained field-scoped search,
//! page-by-page navigation, and a shareable URL (`?q=…&scope=…&page=…`).

use std::cell::RefCell;

use hal_sdk::{Field, HalClient, SearchQuery};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement, UrlSearchParams};

const RESULTS_ID: &str = "results";
const STATUS_ID: &str = "status";
const INPUT_ID: &str = "query";
const SCOPE_ID: &str = "scope";
const PAGER_ID: &str = "pager";
const PREV_ID: &str = "prev";
const NEXT_ID: &str = "next";
const PAGEINFO_ID: &str = "pageinfo";

const GITHUB_URL: &str = "https://github.com/thepriben/hal-sdk";
const CRATES_URL: &str = "https://crates.io/crates/hal-sdk";
const DOCS_URL: &str = "https://docs.rs/hal-sdk";
const BOOK_URL: &str =
    "https://www.editions-eni.fr/livre/rust-developpez-des-programmes-robustes-et-securises-9782409035289";

const PAGE_SIZE: u32 = 20;
const DEBOUNCE_MS: i32 = 300;

const RETURNED_FIELDS: [&str; 7] = [
    "docid",
    "label_s",
    "uri_s",
    "title_s",
    "authFullName_s",
    "producedDate_s",
    "docType_s",
];

/// The state currently shown, so the pager buttons know where to go next.
#[derive(Clone, Default)]
struct Current {
    q: String,
    scope: String,
    page: u32,
    num_found: i64,
}

/// A scheduled debounce timer: its handle (to cancel it) and the closure it runs.
type DebounceTimer = (i32, Closure<dyn FnMut()>);

thread_local! {
    static CURRENT: RefCell<Current> = RefCell::new(Current::default());
    static DEBOUNCE: RefCell<Option<DebounceTimer>> = const { RefCell::new(None) };
}

fn main() {
    console_error_panic_hook::set_once();
    let document = web_sys::window()
        .and_then(|w| w.document())
        .expect("a browser document is required");
    build_ui(&document);
    restore_from_url(&document);
}

fn build_ui(document: &Document) {
    let body = document.body().expect("document should have a body");

    let root = el(document, "main");
    root.set_class_name("app");

    let title = el(document, "h1");
    title.set_text_content(Some("hal-sdk"));
    root.append_child(&title).unwrap();

    let subtitle = el(document, "p");
    subtitle.set_class_name("subtitle");
    subtitle.set_text_content(Some(
        "Search HAL — the French open-access repository of scholarly documents — \
         straight from your browser. This page runs the hal-sdk Rust crate compiled \
         to WebAssembly.",
    ));
    root.append_child(&subtitle).unwrap();

    let form = el(document, "form");
    form.set_class_name("search");

    let scope = el(document, "select");
    scope.set_id(SCOPE_ID);
    for (value, label) in [
        ("all", "All fields"),
        ("title", "Title"),
        ("author", "Author"),
        ("abstract", "Abstract"),
        ("keyword", "Keyword"),
    ] {
        let option = el(document, "option");
        option.set_attribute("value", value).unwrap();
        option.set_text_content(Some(label));
        scope.append_child(&option).unwrap();
    }
    form.append_child(&scope).unwrap();

    let input = el(document, "input");
    input.set_id(INPUT_ID);
    input.set_attribute("type", "search").unwrap();
    input
        .set_attribute("placeholder", "e.g. programmation")
        .unwrap();
    input.set_attribute("autocomplete", "off").unwrap();
    form.append_child(&input).unwrap();

    let button = el(document, "button");
    button.set_attribute("type", "submit").unwrap();
    button.set_text_content(Some("Search"));
    form.append_child(&button).unwrap();

    root.append_child(&form).unwrap();

    let status = el(document, "p");
    status.set_id(STATUS_ID);
    status.set_class_name("status");
    status.set_text_content(Some("Type to search — results appear as you type."));
    root.append_child(&status).unwrap();

    let results = el(document, "section");
    results.set_id(RESULTS_ID);
    root.append_child(&results).unwrap();

    build_pager(document, &root);
    build_footer(document, &root);

    body.append_child(&root).unwrap();

    wire_events(document, &form, &input, &scope);
}

fn build_pager(document: &Document, root: &Element) {
    let pager = el(document, "nav");
    pager.set_id(PAGER_ID);
    pager.set_class_name("pager");
    pager.set_attribute("hidden", "").unwrap();

    let prev = el(document, "button");
    prev.set_id(PREV_ID);
    prev.set_attribute("type", "button").unwrap();
    prev.set_text_content(Some("← Previous"));
    pager.append_child(&prev).unwrap();

    let info = el(document, "span");
    info.set_id(PAGEINFO_ID);
    info.set_class_name("pageinfo");
    pager.append_child(&info).unwrap();

    let next = el(document, "button");
    next.set_id(NEXT_ID);
    next.set_attribute("type", "button").unwrap();
    next.set_text_content(Some("Next →"));
    pager.append_child(&next).unwrap();

    root.append_child(&pager).unwrap();
}

fn build_footer(document: &Document, root: &Element) {
    let footer = el(document, "footer");

    let links = [
        (GITHUB_URL, "Source on GitHub"),
        (CRATES_URL, "crates.io"),
        (DOCS_URL, "docs.rs"),
        (BOOK_URL, "Book — Rust (ENI, 1st edition, 2022)"),
    ];

    for (index, (href, label)) in links.iter().enumerate() {
        if index > 0 {
            let sep = el(document, "span");
            sep.set_class_name("sep");
            sep.set_text_content(Some(" · "));
            footer.append_child(&sep).unwrap();
        }

        let link = el(document, "a");
        link.set_attribute("href", href).unwrap();
        link.set_attribute("target", "_blank").unwrap();
        link.set_attribute("rel", "noopener").unwrap();
        link.set_text_content(Some(label));
        footer.append_child(&link).unwrap();
    }

    root.append_child(&footer).unwrap();
}

fn wire_events(document: &Document, form: &Element, input: &Element, scope: &Element) {
    // Submitting the form runs an immediate search (page 0), pushing a history entry.
    on(form, "submit", {
        let document = document.clone();
        move |event: Event| {
            event.prevent_default();
            let (q, scope) = read_inputs(&document);
            navigate(&document, &q, &scope, 0, Nav::Push);
        }
    });

    // Typing runs a debounced ("quick") search; it only replaces the URL to avoid
    // flooding the history with one entry per keystroke.
    on(input, "input", {
        let document = document.clone();
        move |_event: Event| schedule_search(&document)
    });

    // Changing the scope re-runs the search right away.
    on(scope, "change", {
        let document = document.clone();
        move |_event: Event| {
            let (q, scope) = read_inputs(&document);
            navigate(&document, &q, &scope, 0, Nav::Push);
        }
    });

    // Page navigation.
    if let Some(prev) = document.get_element_by_id(PREV_ID) {
        on(&prev, "click", {
            let document = document.clone();
            move |_event: Event| {
                let c = current();
                if c.page > 0 {
                    navigate(&document, &c.q, &c.scope, c.page - 1, Nav::Push);
                }
            }
        });
    }
    if let Some(next) = document.get_element_by_id(NEXT_ID) {
        on(&next, "click", {
            let document = document.clone();
            move |_event: Event| {
                let c = current();
                let total_pages = (c.num_found as u64).div_ceil(PAGE_SIZE as u64).max(1);
                if (c.page as u64) + 1 < total_pages {
                    navigate(&document, &c.q, &c.scope, c.page + 1, Nav::Push);
                }
            }
        });
    }

    // Back/forward buttons restore the state encoded in the URL.
    let window = web_sys::window().expect("window");
    let document = document.clone();
    let popstate = Closure::<dyn FnMut(Event)>::new(move |_event: Event| {
        restore_from_url(&document);
    });
    window
        .add_event_listener_with_callback("popstate", popstate.as_ref().unchecked_ref())
        .unwrap();
    popstate.forget();
}

/// How the URL should be updated for a navigation.
#[derive(Clone, Copy)]
enum Nav {
    Push,
    Replace,
}

/// Debounce: cancel any pending search and schedule a new one shortly after.
fn schedule_search(document: &Document) {
    let window = web_sys::window().expect("window");

    DEBOUNCE.with(|slot| {
        if let Some((handle, _)) = slot.borrow_mut().take() {
            window.clear_timeout_with_handle(handle);
        }

        let document = document.clone();
        let callback = Closure::<dyn FnMut()>::new(move || {
            let (q, scope) = read_inputs(&document);
            navigate(&document, &q, &scope, 0, Nav::Replace);
        });
        let handle = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                DEBOUNCE_MS,
            )
            .expect("failed to set timeout");
        *slot.borrow_mut() = Some((handle, callback));
    });
}

/// Update the URL and run the search (without touching the form fields).
fn navigate(document: &Document, q: &str, scope: &str, page: u32, nav: Nav) {
    set_url(nav, q, scope, page);
    run_search(document, q.to_owned(), scope.to_owned(), page);
}

/// Read the URL, reflect it into the form fields, and run the search.
fn restore_from_url(document: &Document) {
    let (q, scope, page) = parse_url();
    set_value(document, INPUT_ID, &q);
    set_value(document, SCOPE_ID, &scope);
    if q.trim().is_empty() {
        set_status(document, "Type to search — results appear as you type.");
        clear_results(document);
        hide_pager(document);
    } else {
        run_search(document, q, scope, page);
    }
}

fn run_search(document: &Document, q: String, scope: String, page: u32) {
    let q = q.trim().to_owned();
    if q.is_empty() {
        set_status(document, "Type something to search for.");
        clear_results(document);
        hide_pager(document);
        return;
    }

    let search = build_query(&scope, &q, page);
    let document = document.clone();
    set_status(&document, "Searching…");

    spawn_local(async move {
        let client = HalClient::new();
        match client.search(&search).await {
            Ok(results) => {
                let num_found = results.num_found();
                set_current(Current {
                    q: q.clone(),
                    scope: scope.clone(),
                    page,
                    num_found,
                });
                set_status(
                    &document,
                    &format!(
                        "{num_found} documents found — showing {}.",
                        results.docs().len()
                    ),
                );
                render_results(&document, results.docs());
                update_pager(&document, page, num_found);
            }
            Err(reason) => {
                set_status(&document, &format!("Error: {reason}"));
                hide_pager(&document);
            }
        }
    });
}

fn build_query(scope: &str, query: &str, page: u32) -> SearchQuery {
    let base = match scope {
        "title" => SearchQuery::in_field(Field::Title, query),
        "author" => SearchQuery::in_field(Field::Author, query),
        "abstract" => SearchQuery::in_field(Field::Abstract, query),
        "keyword" => SearchQuery::in_field(Field::Keyword, query),
        _ => SearchQuery::basic(query),
    };
    base.fields(RETURNED_FIELDS).page(page, PAGE_SIZE)
}

fn render_results(document: &Document, docs: &[hal_sdk::HalDoc]) {
    let Some(container) = document.get_element_by_id(RESULTS_ID) else {
        return;
    };
    container.set_inner_html("");

    for doc in docs {
        let card = el(document, "article");
        card.set_class_name("card");

        let heading = el(document, "h2");
        heading.set_text_content(Some(doc.heading().unwrap_or("(untitled)")));
        card.append_child(&heading).unwrap();

        if let Some(authors) = doc.authors() {
            let authors_el = el(document, "p");
            authors_el.set_class_name("authors");
            authors_el.set_text_content(Some(&authors));
            card.append_child(&authors_el).unwrap();
        }

        let meta = el(document, "p");
        meta.set_class_name("meta");
        let mut parts: Vec<String> = Vec::new();
        if let Some(kind) = &doc.doc_type_s {
            parts.push(kind.clone());
        }
        if let Some(date) = &doc.produced_date_s {
            parts.push(date.clone());
        }
        parts.push(format!("HAL id {}", doc.docid));
        meta.set_text_content(Some(&parts.join(" · ")));
        card.append_child(&meta).unwrap();

        if let Some(uri) = &doc.uri_s {
            let link = el(document, "a");
            link.set_attribute("href", uri).unwrap();
            link.set_attribute("target", "_blank").unwrap();
            link.set_attribute("rel", "noopener").unwrap();
            link.set_text_content(Some("Open on HAL →"));
            card.append_child(&link).unwrap();
        }

        container.append_child(&card).unwrap();
    }
}

fn update_pager(document: &Document, page: u32, num_found: i64) {
    let Some(pager) = document.get_element_by_id(PAGER_ID) else {
        return;
    };
    if num_found <= 0 {
        pager.set_attribute("hidden", "").unwrap();
        return;
    }
    pager.remove_attribute("hidden").unwrap();

    let total_pages = ((num_found as u64).div_ceil(PAGE_SIZE as u64)).max(1);
    let human_page = page as u64 + 1;

    if let Some(info) = document.get_element_by_id(PAGEINFO_ID) {
        info.set_text_content(Some(&format!("Page {human_page} of {total_pages}")));
    }
    set_disabled(document, PREV_ID, page == 0);
    set_disabled(document, NEXT_ID, human_page >= total_pages);
}

// --- URL state -------------------------------------------------------------

fn set_url(nav: Nav, q: &str, scope: &str, page: u32) {
    let window = web_sys::window().expect("window");
    let history = window.history().expect("history");

    let params = UrlSearchParams::new().expect("UrlSearchParams");
    params.set("q", q);
    params.set("scope", scope);
    params.set("page", &page.to_string());
    let url = format!("?{}", String::from(params.to_string()));

    let result = match nav {
        Nav::Push => history.push_state_with_url(&JsValue::NULL, "", Some(&url)),
        Nav::Replace => history.replace_state_with_url(&JsValue::NULL, "", Some(&url)),
    };
    let _ = result;
}

fn parse_url() -> (String, String, u32) {
    let window = web_sys::window().expect("window");
    let search = window.location().search().unwrap_or_default();
    let params = UrlSearchParams::new_with_str(&search)
        .unwrap_or_else(|_| UrlSearchParams::new().expect("UrlSearchParams"));
    let q = params.get("q").unwrap_or_default();
    let scope = params.get("scope").unwrap_or_else(|| "all".to_owned());
    let page = params
        .get("page")
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(0);
    (q, scope, page)
}

// --- small DOM helpers ------------------------------------------------------

fn read_inputs(document: &Document) -> (String, String) {
    (value(document, INPUT_ID), value(document, SCOPE_ID))
}

fn current() -> Current {
    CURRENT.with(|c| c.borrow().clone())
}

fn set_current(new: Current) {
    CURRENT.with(|c| *c.borrow_mut() = new);
}

/// Attach an event listener, leaking the closure so it lives for the app's lifetime.
fn on<F>(target: &Element, event: &str, handler: F)
where
    F: FnMut(Event) + 'static,
{
    let closure = Closure::<dyn FnMut(Event)>::new(handler);
    target
        .add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

fn value(document: &Document, id: &str) -> String {
    let Some(element) = document.get_element_by_id(id) else {
        return String::new();
    };
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        return input.value();
    }
    if let Some(select) = element.dyn_ref::<HtmlSelectElement>() {
        return select.value();
    }
    String::new()
}

fn set_value(document: &Document, id: &str, value: &str) {
    let Some(element) = document.get_element_by_id(id) else {
        return;
    };
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        input.set_value(value);
    } else if let Some(select) = element.dyn_ref::<HtmlSelectElement>() {
        select.set_value(if value.is_empty() { "all" } else { value });
    }
}

fn set_disabled(document: &Document, id: &str, disabled: bool) {
    if let Some(element) = document.get_element_by_id(id) {
        if disabled {
            element.set_attribute("disabled", "").unwrap();
        } else {
            element.remove_attribute("disabled").unwrap();
        }
    }
}

fn set_status(document: &Document, message: &str) {
    if let Some(status) = document.get_element_by_id(STATUS_ID) {
        status.set_text_content(Some(message));
    }
}

fn clear_results(document: &Document) {
    if let Some(container) = document.get_element_by_id(RESULTS_ID) {
        container.set_inner_html("");
    }
}

fn hide_pager(document: &Document) {
    if let Some(pager) = document.get_element_by_id(PAGER_ID) {
        pager.set_attribute("hidden", "").unwrap();
    }
}

fn el(document: &Document, tag: &str) -> Element {
    document
        .create_element(tag)
        .expect("failed to create element")
}
