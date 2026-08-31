use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::doc::HalDoc;

/// The top-level payload returned by the HAL search endpoint.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    /// The main response body (result count, offset and documents).
    pub response: ResponseBody,
    /// Facet counts, present only when the query requested facets.
    #[serde(default)]
    pub facet_counts: Option<FacetCounts>,
}

impl SearchResponse {
    /// Total number of documents matching the query (across all pages).
    pub fn num_found(&self) -> i64 {
        self.response.num_found
    }

    /// The offset (`start`) of the current page.
    pub fn start(&self) -> i64 {
        self.response.start
    }

    /// The documents on the current page.
    pub fn docs(&self) -> &[HalDoc] {
        &self.response.docs
    }

    /// The facet counts, if any were requested.
    pub fn facets(&self) -> Option<&FacetCounts> {
        self.facet_counts.as_ref()
    }
}

/// The `response` object of a HAL search payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResponseBody {
    /// Total number of matching documents.
    #[serde(rename = "numFound")]
    pub num_found: i64,
    /// Offset of the first returned document (pagination cursor).
    pub start: i64,
    /// The documents returned for this page.
    pub docs: Vec<HalDoc>,
}

/// Solr-style facet counts.
///
/// HAL returns facet fields as flat arrays alternating value and count, e.g.
/// `["ART", 1234, "COMM", 567]`. [`FacetCounts::field`] turns them into pairs.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FacetCounts {
    /// Raw facet fields keyed by field name.
    #[serde(default)]
    pub facet_fields: HashMap<String, Vec<serde_json::Value>>,
}

impl FacetCounts {
    /// Return the `(value, count)` pairs for a facet field, in the order HAL sent them.
    // `chunks_exact(2)` states the intent more plainly than `as_chunks::<2>()` and works on
    // older toolchains, so the lint that suggests the newer API is waived. The outer `allow`
    // keeps this file compiling warning-free on compilers that predate that lint.
    #[allow(unknown_lints)]
    #[allow(clippy::chunks_exact_to_as_chunks)]
    pub fn field(&self, name: &str) -> Vec<(String, i64)> {
        let Some(raw) = self.facet_fields.get(name) else {
            return Vec::new();
        };
        raw.chunks_exact(2)
            .filter_map(|pair| {
                let value = pair[0].as_str()?.to_owned();
                let count = pair[1].as_i64()?;
                Some((value, count))
            })
            .collect()
    }
}
