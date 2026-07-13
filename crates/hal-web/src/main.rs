//! A minimal, framework-free WebAssembly front-end for `hal-sdk`.
//!
//! The whole application is written in Rust: it builds the DOM with `web-sys`
//! and calls the SDK directly from the browser (the HAL API allows
//! cross-origin requests, so no backend is required). Build it with `trunk`.

use std::cell::RefCell;

use hal_sdk::{Field, HalClient, SearchQuery};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement};

const RESULTS_ID: &str = "results";
const STATUS_ID: &str = "status";
const INPUT_ID: &str = "query";
const SCOPE_ID: &str = "scope";
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

/// A scheduled debounce timer: its handle (to cancel it) and the closure it runs
/// (kept alive until it fires or is replaced).
type DebounceTimer = (i32, Closure<dyn FnMut()>);

thread_local! {
    // Keeps the pending debounce timer alive between keystrokes.
    static DEBOUNCE: RefCell<Option<DebounceTimer>> = const { RefCell::new(None) };
}

fn main() {
    console_error_panic_hook::set_once();
    let document = web_sys::window()
        .and_then(|w| w.document())
        .expect("a browser document is required");
    build_ui(&document);
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

    body.append_child(&root).unwrap();

    wire_events(document, &form, &input, &scope);
}

fn wire_events(document: &Document, form: &Element, input: &Element, scope: &Element) {
    // Submitting the form triggers an immediate search (no page reload).
    on(form, "submit", {
        let document = document.clone();
        move |event: Event| {
            event.prevent_default();
            search_now(&document);
        }
    });

    // Typing triggers a debounced ("quick") search.
    on(input, "input", {
        let document = document.clone();
        move |_event: Event| schedule_search(&document)
    });

    // Changing the scope re-runs the search right away.
    on(scope, "change", {
        let document = document.clone();
        move |_event: Event| search_now(&document)
    });
}

/// Debounce: cancel any pending search and schedule a new one shortly after.
fn schedule_search(document: &Document) {
    let window = web_sys::window().expect("window");

    DEBOUNCE.with(|slot| {
        if let Some((handle, _)) = slot.borrow_mut().take() {
            window.clear_timeout_with_handle(handle);
        }

        let document = document.clone();
        let callback = Closure::<dyn FnMut()>::new(move || search_now(&document));
        let handle = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                DEBOUNCE_MS,
            )
            .expect("failed to set timeout");
        *slot.borrow_mut() = Some((handle, callback));
    });
}

fn search_now(document: &Document) {
    let query = value(document, INPUT_ID);
    let query = query.trim().to_owned();
    let scope = value(document, SCOPE_ID);

    if query.is_empty() {
        set_status(document, "Type something to search for.");
        clear_results(document);
        return;
    }

    let search = build_query(&scope, &query);

    let document = document.clone();
    set_status(&document, "Searching…");

    spawn_local(async move {
        let client = HalClient::new();
        match client.search(&search).await {
            Ok(results) => {
                set_status(
                    &document,
                    &format!(
                        "{} documents found — showing {}.",
                        results.num_found(),
                        results.docs().len()
                    ),
                );
                render_results(&document, results.docs());
            }
            Err(reason) => set_status(&document, &format!("Error: {reason}")),
        }
    });
}

fn build_query(scope: &str, query: &str) -> SearchQuery {
    let base = match scope {
        "title" => SearchQuery::in_field(Field::Title, query),
        "author" => SearchQuery::in_field(Field::Author, query),
        "abstract" => SearchQuery::in_field(Field::Abstract, query),
        "keyword" => SearchQuery::in_field(Field::Keyword, query),
        _ => SearchQuery::basic(query),
    };
    base.fields(RETURNED_FIELDS).rows(PAGE_SIZE)
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

fn el(document: &Document, tag: &str) -> Element {
    document
        .create_element(tag)
        .expect("failed to create element")
}
