use crate::error::HalError;
use crate::query::SearchQuery;
use crate::response::SearchResponse;

/// The default HAL search endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.archives-ouvertes.fr/search/";

/// An asynchronous client for the HAL search API.
///
/// The client is cheap to clone and holds a reusable `reqwest` client. It works
/// on native targets and on `wasm32` (where it uses the browser `fetch` API).
///
/// ```no_run
/// use hal_sdk::HalClient;
///
/// # async fn run() -> Result<(), hal_sdk::HalError> {
/// let client = HalClient::new();
/// let results = client.basic_search("rust").await?;
/// println!("{} results", results.num_found());
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct HalClient {
    base_url: String,
    http: reqwest::Client,
}

impl Default for HalClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HalClient {
    /// Create a client pointing at the public HAL search endpoint.
    pub fn new() -> Self {
        HalClient {
            base_url: DEFAULT_BASE_URL.to_owned(),
            http: reqwest::Client::new(),
        }
    }

    /// Create a client pointing at a custom base URL (useful for tests or mirrors).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        HalClient {
            base_url: base_url.into(),
            http: reqwest::Client::new(),
        }
    }

    /// The base URL this client targets.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Run a basic search over all fields, matching the original chapter-16 behaviour.
    pub async fn basic_search(&self, query: &str) -> Result<SearchResponse, HalError> {
        self.search(&SearchQuery::basic(query)).await
    }

    /// Run an arbitrary [`SearchQuery`].
    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResponse, HalError> {
        let response = self
            .http
            .get(&self.base_url)
            .query(&query.to_params())
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let mut body = body;
            body.truncate(500);
            return Err(HalError::Api {
                status: status.as_u16(),
                body,
            });
        }

        Ok(serde_json::from_str(&body)?)
    }
}
