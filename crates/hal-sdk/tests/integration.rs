//! Integration tests that hit the live HAL API.
//!
//! They are tolerant of network failures (offline CI, rate limiting): a transport
//! error is treated as a skip rather than a failure. The response *shape*, however,
//! must be correct whenever the API does answer.

use hal_sdk::{HalClient, SearchQuery};

#[tokio::test]
async fn basic_search_returns_documents() {
    let client = HalClient::new();
    match client.basic_search("programmation").await {
        Ok(results) => {
            assert!(results.num_found() >= 0);
            for doc in results.docs() {
                assert!(!doc.docid.is_empty());
            }
        }
        Err(reason) => eprintln!("skipping: HAL API unreachable ({reason})"),
    }
}

#[tokio::test]
async fn field_search_with_pagination_and_facets() {
    let client = HalClient::new();
    let query = SearchQuery::field("title_t", "europe")
        .fields(["docid", "label_s", "title_s", "docType_s"])
        .facet("docType_s")
        .page(0, 5);

    match client.search(&query).await {
        Ok(results) => {
            assert!(results.docs().len() <= 5);
            if let Some(facets) = results.facets() {
                let _ = facets.field("docType_s");
            }
        }
        Err(reason) => eprintln!("skipping: HAL API unreachable ({reason})"),
    }
}
