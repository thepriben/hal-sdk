# hal-sdk

[![crates.io](https://img.shields.io/crates/v/hal-sdk.svg)](https://crates.io/crates/hal-sdk)
[![docs.rs](https://img.shields.io/docsrs/hal-sdk)](https://docs.rs/hal-sdk)

An asynchronous Rust SDK for the [HAL](https://hal.science) open-archive search API.

## What is HAL?

HAL (*Hyper Articles en Ligne*) is a French open-access repository operated by the
[CCSD](https://www.ccsd.cnrs.fr/). Researchers deposit scholarly documents there —
articles, preprints, theses, conference papers and reports — so that they stay freely
available to everyone. It holds several million records, many in French, though the
archive is multilingual and international.

HAL exposes a public HTTP search API (backed by Apache Solr), documented at
<https://api.archives-ouvertes.fr/docs/search>. This crate wraps that API in idiomatic,
strongly typed Rust.

## Features

- Asynchronous client built on `reqwest`.
- Works on native targets **and** on `wasm32` (in the browser it uses `fetch`).
- A query builder covering every HAL search mode: basic search, search within a field,
  several terms in a field, proximity search, field selection (`fl`), pagination and facets.
- Typed results with the most common fields, plus an `extra` map for anything else.

## Install

```toml
[dependencies]
hal-sdk = "0.2"
```

## Usage

```rust
use hal_sdk::{HalClient, SearchQuery};

#[tokio::main]
async fn main() -> Result<(), hal_sdk::HalError> {
    let client = HalClient::new();

    // Basic search over all fields.
    let results = client.basic_search("programmation").await?;
    println!("{} documents found", results.num_found());

    // A richer query: search a field, pick returned fields, paginate, add a facet.
    let query = SearchQuery::field("title_t", "europe")
        .fields(["docid", "label_s", "title_s", "authFullName_s", "producedDate_s"])
        .facet("docType_s")
        .page(0, 20);

    let results = client.search(&query).await?;
    for doc in results.docs() {
        println!("{doc}");
    }
    Ok(())
}
```

## Companion project

This crate is the companion project of chapter 15 of the book *Rust* by Benoît Prieur
(éditions ENI, second edition). It brings up to date the crate published with chapter 16
of the first edition ("Projet final : coder et publier une caisse"), which shipped a
minimal, blocking, basic-search-only library. This version is a complete, asynchronous,
WebAssembly-capable SDK.

The second edition ships a second companion project, a scientific calculator built
around the [`calc-core`](https://crates.io/crates/calc-core) crate:
<https://github.com/thepriben/calculatrice-sci>.

## License

Licensed under either of MIT or Apache-2.0, at your option.
