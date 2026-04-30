use crate::{Error, search::params::encode_search_params};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchFilter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchQuery {
    pub query: String,
    pub filter: Option<SearchFilter>,
    pub limit: usize,
    pub ignore_spelling: bool,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            filter: None,
            limit: 20,
            ignore_spelling: false,
        }
    }

    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn ignore_spelling(mut self) -> Self {
        self.ignore_spelling = true;
        self
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.query.trim().is_empty() {
            return Err(Error::InvalidInput("query must not be blank".to_owned()));
        }

        if self.limit == 0 {
            return Err(Error::InvalidInput(
                "limit must be greater than zero".to_owned(),
            ));
        }

        Ok(())
    }

    pub fn encoded_params(&self) -> Option<String> {
        encode_search_params(self.filter, self.ignore_spelling)
    }
}
