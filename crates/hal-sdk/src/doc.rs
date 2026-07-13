use std::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

/// A single document returned by a HAL search.
///
/// HAL is a Solr index with a very large, evolving schema. Only the most common
/// fields are typed here; any other field requested through
/// [`SearchQuery::fields`](crate::SearchQuery::fields) is preserved in [`HalDoc::extra`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HalDoc {
    /// The HAL internal document identifier.
    ///
    /// HAL returns this either as a number or as a string depending on the
    /// endpoint and version; it is always normalised to a `String` here.
    #[serde(default, deserialize_with = "string_or_number")]
    pub docid: String,

    /// A ready-to-display citation-style label (default field of a basic search).
    #[serde(default)]
    pub label_s: Option<String>,

    /// The canonical URI of the document on HAL (e.g. `https://hal.science/…`).
    #[serde(default)]
    pub uri_s: Option<String>,

    /// The document title(s). Requested with `fl=title_s`.
    #[serde(default)]
    pub title_s: Option<Vec<String>>,

    /// The full names of the authors. Requested with `fl=authFullName_s`.
    #[serde(default, rename = "authFullName_s")]
    pub auth_full_name_s: Option<Vec<String>>,

    /// The production date, as a string (e.g. `2026-06-04`). Requested with `fl=producedDate_s`.
    #[serde(default)]
    pub produced_date_s: Option<String>,

    /// The document type (e.g. `ART`, `COMM`, `THESE`). Requested with `fl=docType_s`.
    #[serde(default)]
    pub doc_type_s: Option<String>,

    /// Any other field returned by HAL but not modelled above.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl HalDoc {
    /// The first title, if any (HAL stores titles as an array).
    pub fn title(&self) -> Option<&str> {
        self.title_s.as_ref()?.first().map(String::as_str)
    }

    /// The authors joined with `", "`, if any.
    pub fn authors(&self) -> Option<String> {
        self.auth_full_name_s.as_ref().map(|a| a.join(", "))
    }

    /// The best human-readable heading available: the title if present,
    /// otherwise the citation label.
    pub fn heading(&self) -> Option<&str> {
        self.title().or(self.label_s.as_deref())
    }
}

impl fmt::Display for HalDoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.heading() {
            Some(heading) => write!(f, "{} ({})", heading, self.docid),
            None => write!(f, "{}", self.docid),
        }
    }
}

/// Accept both a JSON string and a JSON number, normalising to `String`.
fn string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrNumber;

    impl<'de> Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a string or an integer")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
            Ok(v.to_owned())
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> {
            Ok(v)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<String, E> {
            Ok(v.to_string())
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<String, E> {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}
