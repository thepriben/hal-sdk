/// A HAL search field usable for fine-grained (field-scoped) searches.
///
/// These map to the Solr *text* fields exposed by HAL. Use
/// [`Field::name`] to get the raw field name, or pass a [`Field`] to
/// [`SearchQuery::in_field`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The document title (`title_t`).
    Title,
    /// The author full names (`authFullName_t`).
    Author,
    /// The abstract (`abstract_t`).
    Abstract,
    /// The keywords (`keyword_t`).
    Keyword,
}

impl Field {
    /// The raw HAL/Solr field name.
    pub fn name(self) -> &'static str {
        match self {
            Field::Title => "title_t",
            Field::Author => "authFullName_t",
            Field::Abstract => "abstract_t",
            Field::Keyword => "keyword_t",
        }
    }
}

/// A builder for HAL search queries.
///
/// It maps directly onto the Solr query parameters used by the HAL API and
/// covers every search mode listed in the HAL documentation:
///
/// * **basic search** — [`SearchQuery::basic`];
/// * **search within a field** — [`SearchQuery::field`] / [`SearchQuery::in_field`];
/// * **several terms within a field** — [`SearchQuery::field_terms`] (OR) /
///   [`SearchQuery::field_all_terms`] (AND);
/// * **proximity search** — [`SearchQuery::proximity`];
/// * **field selection** (`fl`) — [`SearchQuery::fields`];
/// * **pagination** — [`SearchQuery::rows`] / [`SearchQuery::start`] / [`SearchQuery::page`];
/// * **facets** — [`SearchQuery::facet`].
///
/// The builder methods consume and return `self`, so they can be chained.
#[derive(Clone, Debug)]
pub struct SearchQuery {
    q: String,
    fields: Vec<String>,
    rows: Option<u32>,
    start: Option<u32>,
    sort: Option<String>,
    facet_fields: Vec<String>,
}

impl SearchQuery {
    /// A basic search over all fields (the `q` parameter is used as-is).
    ///
    /// This is the exact behaviour of the original chapter-16 library.
    #[must_use]
    pub fn basic(query: impl Into<String>) -> Self {
        SearchQuery {
            q: query.into(),
            fields: Vec::new(),
            rows: None,
            start: None,
            sort: None,
            facet_fields: Vec::new(),
        }
    }

    /// Search for a value within a specific field, e.g. `field("title_t", "europe")`
    /// produces `q=title_t:europe`.
    #[must_use]
    pub fn field(field: &str, value: &str) -> Self {
        Self::basic(format!("{field}:{value}"))
    }

    /// Fine-grained search scoped to a typed [`Field`].
    ///
    /// A multi-word value is grouped so that every word is searched within the
    /// field, e.g. `in_field(Field::Title, "open science")` produces
    /// `q=title_t:(open science)`. An empty value falls back to a basic search.
    #[must_use]
    pub fn in_field(field: Field, value: &str) -> Self {
        let words: Vec<&str> = value.split_whitespace().collect();
        if words.is_empty() {
            Self::basic(value.to_owned())
        } else {
            Self::field_terms(field.name(), words)
        }
    }

    /// Search for several terms within a single field, matching **any** of them
    /// (Solr `OR`), e.g. `field_terms("title_t", ["economic", "policy"])`
    /// produces `q=title_t:(economic policy)`.
    #[must_use]
    pub fn field_terms<I, S>(field: &str, terms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let joined = terms
            .into_iter()
            .map(|t| t.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(" ");
        Self::basic(format!("{field}:({joined})"))
    }

    /// Search for several terms within a single field, matching **all** of them
    /// (Solr `AND`), e.g. `field_all_terms("title_t", ["economic", "policy"])`
    /// produces `q=title_t:(economic AND policy)`.
    #[must_use]
    pub fn field_all_terms<I, S>(field: &str, terms: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let joined = terms
            .into_iter()
            .map(|t| t.as_ref().to_owned())
            .collect::<Vec<_>>()
            .join(" AND ");
        Self::basic(format!("{field}:({joined})"))
    }

    /// Proximity search: find `phrase` within `distance` words inside `field`,
    /// e.g. `proximity("title_t", "economic policy", 3)` produces
    /// `q=title_t:"economic policy"~3`.
    #[must_use]
    pub fn proximity(field: &str, phrase: &str, distance: u32) -> Self {
        Self::basic(format!("{field}:\"{phrase}\"~{distance}"))
    }

    /// Restrict the returned fields (the Solr `fl` parameter).
    #[must_use]
    pub fn fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.fields = fields.into_iter().map(|f| f.as_ref().to_owned()).collect();
        self
    }

    /// Set the number of rows (documents) to return.
    #[must_use]
    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = Some(rows);
        self
    }

    /// Set the offset of the first returned document.
    #[must_use]
    pub fn start(mut self, start: u32) -> Self {
        self.start = Some(start);
        self
    }

    /// Convenience pagination: page `page` (0-based) with `per_page` results each.
    #[must_use]
    pub fn page(mut self, page: u32, per_page: u32) -> Self {
        self.rows = Some(per_page);
        self.start = Some(page * per_page);
        self
    }

    /// Set the sort clause (the Solr `sort` parameter), e.g. `"producedDate_s desc"`.
    #[must_use]
    pub fn sort(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }

    /// Add a facet field. Calling this at least once enables faceting.
    #[must_use]
    pub fn facet(mut self, field: impl Into<String>) -> Self {
        self.facet_fields.push(field.into());
        self
    }

    /// Serialise the query into HAL/Solr URL parameters.
    ///
    /// The output type is compatible with `reqwest`'s `.query(..)`.
    #[must_use]
    pub fn to_params(&self) -> Vec<(String, String)> {
        let mut params = vec![
            ("q".to_owned(), self.q.clone()),
            ("wt".to_owned(), "json".to_owned()),
        ];

        if !self.fields.is_empty() {
            params.push(("fl".to_owned(), self.fields.join(",")));
        }
        if let Some(rows) = self.rows {
            params.push(("rows".to_owned(), rows.to_string()));
        }
        if let Some(start) = self.start {
            params.push(("start".to_owned(), start.to_string()));
        }
        if let Some(sort) = &self.sort {
            params.push(("sort".to_owned(), sort.clone()));
        }
        if !self.facet_fields.is_empty() {
            params.push(("facet".to_owned(), "true".to_owned()));
            for field in &self.facet_fields {
                params.push(("facet.field".to_owned(), field.clone()));
            }
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
        params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    #[test]
    fn basic_query() {
        let params = SearchQuery::basic("europe").to_params();
        assert_eq!(param(&params, "q"), Some("europe"));
        assert_eq!(param(&params, "wt"), Some("json"));
    }

    #[test]
    fn field_query() {
        let params = SearchQuery::field("title_t", "europe").to_params();
        assert_eq!(param(&params, "q"), Some("title_t:europe"));
    }

    #[test]
    fn field_terms_query() {
        let params = SearchQuery::field_terms("title_t", ["economic", "policy"]).to_params();
        assert_eq!(param(&params, "q"), Some("title_t:(economic policy)"));
    }

    #[test]
    fn proximity_query() {
        let params = SearchQuery::proximity("title_t", "economic policy", 3).to_params();
        assert_eq!(param(&params, "q"), Some("title_t:\"economic policy\"~3"));
    }

    #[test]
    fn pagination_and_fields() {
        let params = SearchQuery::basic("europe")
            .fields(["docid", "label_s"])
            .page(2, 10)
            .to_params();
        assert_eq!(param(&params, "fl"), Some("docid,label_s"));
        assert_eq!(param(&params, "rows"), Some("10"));
        assert_eq!(param(&params, "start"), Some("20"));
    }

    #[test]
    fn field_all_terms_uses_and() {
        let params = SearchQuery::field_all_terms("title_t", ["economic", "policy"]).to_params();
        assert_eq!(param(&params, "q"), Some("title_t:(economic AND policy)"));
    }

    #[test]
    fn in_field_groups_words() {
        let params = SearchQuery::in_field(Field::Title, "open science").to_params();
        assert_eq!(param(&params, "q"), Some("title_t:(open science)"));

        let single = SearchQuery::in_field(Field::Author, "dupont").to_params();
        assert_eq!(param(&single, "q"), Some("authFullName_t:(dupont)"));
    }

    #[test]
    fn facets_enabled() {
        let params = SearchQuery::basic("europe").facet("docType_s").to_params();
        assert_eq!(param(&params, "facet"), Some("true"));
        assert_eq!(param(&params, "facet.field"), Some("docType_s"));
    }
}
