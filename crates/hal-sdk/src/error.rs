use std::error::Error;
use std::fmt;

/// Errors that can occur while talking to the HAL API.
#[derive(Debug)]
pub enum HalError {
    /// The HTTP request itself failed (network error, timeout, TLS, …).
    Request(reqwest::Error),
    /// The response body could not be decoded into the expected structure.
    Json(serde_json::Error),
    /// The API returned an unexpected HTTP status code.
    Api {
        /// The HTTP status code returned by HAL.
        status: u16,
        /// The (possibly truncated) response body, useful for debugging.
        body: String,
    },
}

impl fmt::Display for HalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HalError::Request(err) => write!(f, "HTTP request error: {err}"),
            HalError::Json(err) => write!(f, "JSON decoding error: {err}"),
            HalError::Api { status, body } => {
                write!(f, "HAL API returned status {status}: {body}")
            }
        }
    }
}

impl Error for HalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HalError::Request(err) => Some(err),
            HalError::Json(err) => Some(err),
            HalError::Api { .. } => None,
        }
    }
}

impl From<reqwest::Error> for HalError {
    fn from(err: reqwest::Error) -> Self {
        HalError::Request(err)
    }
}

impl From<serde_json::Error> for HalError {
    fn from(err: serde_json::Error) -> Self {
        HalError::Json(err)
    }
}
