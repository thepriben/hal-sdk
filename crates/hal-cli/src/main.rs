//! A tiny consumer of `hal-sdk`, the updated equivalent of the chapter-16
//! `simpletestclient`.
//!
//! Usage:
//!   hal-cli <query> [rows]
//!
//! Example:
//!   hal-cli programmation 10

use std::process::ExitCode;

use hal_sdk::{HalClient, SearchQuery};

#[tokio::main]
async fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let query = args.next().unwrap_or_else(|| "programmation".to_owned());
    let rows: u32 = args.next().and_then(|r| r.parse().ok()).unwrap_or(10);

    let client = HalClient::new();
    let search = SearchQuery::basic(&query)
        .fields([
            "docid",
            "label_s",
            "uri_s",
            "title_s",
            "authFullName_s",
            "producedDate_s",
            "docType_s",
        ])
        .rows(rows);

    match client.search(&search).await {
        Ok(results) => {
            println!("{} documents found for \"{query}\"", results.num_found());
            println!("showing {}:\n", results.docs().len());
            for doc in results.docs() {
                let heading = doc.heading().unwrap_or("(untitled)");
                println!("• [{}] {heading}", doc.docid);
                if let Some(authors) = doc.authors() {
                    println!("    {authors}");
                }
                if let Some(uri) = &doc.uri_s {
                    println!("    {uri}");
                }
            }
            ExitCode::SUCCESS
        }
        Err(reason) => {
            eprintln!("error: {reason}");
            ExitCode::FAILURE
        }
    }
}
