# hal-sdk

[![crates.io](https://img.shields.io/crates/v/hal-sdk.svg)](https://crates.io/crates/hal-sdk)
[![docs.rs](https://img.shields.io/docsrs/hal-sdk)](https://docs.rs/hal-sdk)
[![license](https://img.shields.io/crates/l/hal-sdk.svg)](#license)

An asynchronous Rust SDK for the [HAL](https://hal.science) open-archive search API,
with a command-line example and a WebAssembly front-end.

## What is HAL?

HAL (*Hyper Articles en Ligne*) is a French open-access repository operated by the
[CCSD](https://www.ccsd.cnrs.fr/). Researchers deposit scholarly documents there —
articles, preprints, theses, conference papers and reports — so that they stay freely
available to everyone. It holds several million records, many in French, though the
archive is multilingual and international. HAL exposes a public HTTP search API
(backed by Apache Solr): <https://api.archives-ouvertes.fr/docs/search>.

## Companion project

This repository is a companion project of the book *Rust* by Benoît Prieur
(éditions ENI). It accompanied chapter 16 of the first edition
("Projet final : coder et publier une caisse"), and remains a companion
project of the second edition.

The original chapter published a minimal, blocking, basic-search-only crate
(`apiarchivesouvertesrust` `0.1`). This version modernises it into a complete,
asynchronous, WebAssembly-capable SDK, plus a small web application built on top of it.

## Workspace layout

| Crate                 | Description                                              |
| --------------------- | -------------------------------------------------------- |
| `crates/hal-sdk`      | The SDK library (published on crates.io).                |
| `crates/hal-cli`      | A small command-line client consuming the SDK.           |
| `crates/hal-web`      | A WebAssembly front-end that runs the SDK in the browser. |

## The SDK

```rust
use hal_sdk::{HalClient, SearchQuery};

#[tokio::main]
async fn main() -> Result<(), hal_sdk::HalError> {
    let client = HalClient::new();
    let results = client.basic_search("programmation").await?;
    println!("{} documents found", results.num_found());

    let query = SearchQuery::proximity("title_t", "open science", 2)
        .fields(["docid", "title_s", "authFullName_s"])
        .facet("docType_s")
        .page(0, 20);
    let results = client.search(&query).await?;
    for doc in results.docs() {
        println!("{doc}");
    }
    Ok(())
}
```

The query builder covers every search mode described in the HAL documentation: basic
search, search within a field, several terms in a field, proximity search, field
selection (`fl`), pagination and facets.

## Command-line example

```sh
cargo run -p hal-cli -- europe 10
```

## WebAssembly front-end

The `hal-web` crate is a framework-free front-end written entirely in Rust. It builds
the DOM with `web-sys` and calls the SDK directly from the browser — the HAL API sends
`Access-Control-Allow-Origin: *`, so no backend is required. It offers **quick** search
(debounced, as you type), **fine-grained** search (scoped to title, author, abstract
or keyword), page-by-page navigation, and **shareable URLs** (the query, scope and page
are encoded as `?q=…&scope=…&page=…`).

**Live demo:** <https://thepriben.github.io/hal-sdk/>

It is deployed automatically by the [Pages workflow](.github/workflows/pages.yml) on
every push to `main`. Enable it once in *Settings → Pages → Build and deployment →
Source: GitHub Actions*.

Build and serve it locally with [Trunk](https://trunkrs.dev):

```sh
cargo install trunk                 # once
rustup target add wasm32-unknown-unknown
cd crates/hal-web
trunk serve                         # then open http://localhost:8080
```

If your environment defines `NO_COLOR=1`, invoke Trunk as `env -u NO_COLOR trunk serve`.

## Development

```sh
cargo test          # unit + integration tests (integration tests hit the live API)
cargo clippy --all-targets
```

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
