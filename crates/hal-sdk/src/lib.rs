//! # hal-sdk
//!
//! An asynchronous Rust SDK for the **HAL** open-archive search API.
//!
//! ## What is HAL?
//!
//! HAL (*Hyper Articles en Ligne*) is a French open-access repository operated by
//! the [CCSD](https://www.ccsd.cnrs.fr/). Researchers deposit scholarly documents
//! there — articles, preprints, theses, conference papers, reports — so that they
//! remain freely available to everyone. It hosts millions of records, many of them
//! in French, though the archive is multilingual and international.
//!
//! HAL exposes a public HTTP search API (backed by Apache Solr) documented at
//! <https://api.archives-ouvertes.fr/docs/search>. This crate wraps that API in
//! idiomatic, strongly typed Rust.
//!
//! ## Design
//!
//! The client is `async` and works on **both native targets and `wasm32`**
//! (in the browser it uses the `fetch` API through `reqwest`). The HAL API sends
//! `Access-Control-Allow-Origin: *`, so it can be queried directly from a web page.
//!
//! ## Quick start
//!
//! ```no_run
//! use hal_sdk::HalClient;
//!
//! # async fn run() -> Result<(), hal_sdk::HalError> {
//! let client = HalClient::new();
//! let results = client.basic_search("programmation").await?;
//! println!("{} documents found", results.num_found());
//! for doc in results.docs() {
//!     println!("{} — {:?}", doc.docid, doc.label_s);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Building richer queries
//!
//! [`SearchQuery`] is a builder covering every search mode described in the HAL
//! documentation: search within a field, several terms in a field, proximity
//! search, field selection (`fl`), pagination and facets.
//!
//! ```no_run
//! use hal_sdk::{HalClient, SearchQuery};
//!
//! # async fn run() -> Result<(), hal_sdk::HalError> {
//! let client = HalClient::new();
//!
//! let query = SearchQuery::field("title_t", "europe")
//!     .fields(["docid", "label_s", "title_s", "authFullName_s", "producedDate_s"])
//!     .facet("docType_s")
//!     .page(0, 20);
//!
//! let results = client.search(&query).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## History
//!
//! This crate is the companion project of chapter 16 ("Projet final : coder et
//! publier une caisse") of the book *Rust* by Benoît Prieur, brought up to date:
//! the original chapter shipped a minimal, blocking, basic-search-only library
//! (`apiarchivesouvertesrust` `0.1`). This version modernises it into a complete,
//! asynchronous, WebAssembly-capable SDK.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The crate version, as published on crates.io.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

mod client;
mod doc;
mod error;
mod query;
mod response;

pub use client::HalClient;
pub use doc::HalDoc;
pub use error::HalError;
pub use query::{Field, SearchQuery};
pub use response::{FacetCounts, ResponseBody, SearchResponse};
