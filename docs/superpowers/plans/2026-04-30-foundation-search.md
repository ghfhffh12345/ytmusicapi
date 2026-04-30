# Foundation Search Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first usable Rust vertical slice of `ytmusicapi`: an async, unauthenticated `YtMusic` client with typed public-catalog `search`.

**Architecture:** The crate stays idiomatic Rust and async-first. A small public client orchestrates request validation, visitor-id bootstrap, HTTP transport, and strict typed parsing for the supported `search` filters, using upstream `sigma67/ytmusicapi` `1.12.0` as the behavioral baseline.

**Tech Stack:** Rust 2024, `reqwest` with `json` and `rustls-tls`, `serde`, `serde_json`, `thiserror`, `regex`, `tokio`, `wiremock`

---

## Planned File Structure

**Modify**
- `Cargo.toml`: add runtime and dev dependencies for HTTP, parsing, async tests, and mock HTTP tests.
- `src/lib.rs`: export the public client, query types, result models, and error type.

**Create**
- `src/client.rs`: public `YtMusic` client, builder, and async `search` entrypoint.
- `src/error.rs`: public `Error` enum and helper error constructors.
- `src/model/mod.rs`: public model module wiring.
- `src/model/common.rs`: shared typed substructures such as thumbnails and artist references.
- `src/model/search.rs`: `SearchFilter`, `SearchQuery`, `SearchResult`, and per-variant structs.
- `src/search/mod.rs`: internal search module wiring.
- `src/search/params.rs`: exact upstream-compatible `search` parameter encoding for supported filters and `ignore_spelling`.
- `src/search/request.rs`: request body construction, visitor-id bootstrap, and HTTP POST transport.
- `src/search/parse.rs`: JSON-to-domain translation for supported search results.
- `devtools/capture_search_fixtures.py`: one-off baseline capture utility using Python `ytmusicapi==1.12.0`.
- `tests/client_construction.rs`: public client construction smoke test.
- `tests/search_params.rs`: unit tests for filter and `ignore_spelling` parameter encoding.
- `tests/search_transport.rs`: mock-server tests for visitor-id bootstrap, request body, and error mapping.
- `tests/search_parser.rs`: fixture-based parser tests against captured `1.12.0` raw responses and expected parsed outputs.
- `tests/fixtures/search/raw/default_mixed.json`
- `tests/fixtures/search/raw/songs.json`
- `tests/fixtures/search/raw/videos.json`
- `tests/fixtures/search/raw/albums.json`
- `tests/fixtures/search/raw/artists.json`
- `tests/fixtures/search/raw/playlists.json`
- `tests/fixtures/search/expected/default_mixed.json`
- `tests/fixtures/search/expected/songs.json`
- `tests/fixtures/search/expected/videos.json`
- `tests/fixtures/search/expected/albums.json`
- `tests/fixtures/search/expected/artists.json`
- `tests/fixtures/search/expected/playlists.json`

## Public API Targets

Use these exact public types and signatures throughout implementation:

```rust
pub struct YtMusic { /* private fields */ }

impl YtMusic {
    pub fn new() -> Result<Self, Error>;
    pub fn builder() -> YtMusicBuilder;
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, Error>;
}

pub struct YtMusicBuilder { /* private fields */ }

impl YtMusicBuilder {
    pub fn build(self) -> Result<YtMusic, Error>;
    pub fn http_client(self, client: reqwest::Client) -> Self;
    pub fn base_url(self, base_url: impl Into<String>) -> Self;
    pub fn homepage_url(self, homepage_url: impl Into<String>) -> Self;
}

pub enum SearchFilter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

pub struct SearchQuery {
    pub query: String,
    pub filter: Option<SearchFilter>,
    pub limit: usize,
    pub ignore_spelling: bool,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self;
    pub fn with_filter(self, filter: SearchFilter) -> Self;
    pub fn with_limit(self, limit: usize) -> Self;
    pub fn ignore_spelling(self) -> Self;
    pub fn validate(&self) -> Result<(), Error>;
    pub fn encoded_params(&self) -> Option<String>;
}
```

Use these exact default values:

- `SearchQuery::new(...)` sets `filter = None`, `limit = 20`, `ignore_spelling = false`.
- `YtMusic::new()` uses `https://music.youtube.com/youtubei/v1/` as the API base URL.
- `YtMusic::new()` uses `https://music.youtube.com` as the homepage URL for visitor-id bootstrap.

Use these exact upstream-compatible search params:

- default search, no `ignore_spelling`: no `params` field
- default search with `ignore_spelling`: `EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D`
- `Songs`: `EgWKAQIIAWoMEA4QChADEAQQCRAF`
- `Songs` with `ignore_spelling`: `EgWKAQIIAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D`
- `Videos`: `EgWKAQIQAWoMEA4QChADEAQQCRAF`
- `Videos` with `ignore_spelling`: `EgWKAQIQAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D`
- `Albums`: `EgWKAQIYAWoMEA4QChADEAQQCRAF`
- `Albums` with `ignore_spelling`: `EgWKAQIYAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D`
- `Artists`: `EgWKAQIgAWoMEA4QChADEAQQCRAF`
- `Artists` with `ignore_spelling`: `EgWKAQIgAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D`
- `Playlists`: `Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D`
- `Playlists` with `ignore_spelling`: `Eg-KAQwIABAAGAAgACgBMABCAggBagoQBBADEAkQBRAK`

## Fixture Baseline Decision

Anonymous upstream `ytmusicapi==1.12.0` search responses remain the required behavioral baseline for this slice, but not every filtered search is stable enough to serve as a golden parser fixture in the current environment.

For the remaining tasks in this plan:

- `default_mixed`, `albums`, `artists`, and `playlists` are treated as stable anonymous golden fixtures.
- `songs` and `videos` remain supported by the public Rust request model and transport layer, but they are not required parser-acceptance fixtures in this slice because anonymous upstream responses were empty or low-quality/unreliable.
- Any `songs` or `videos` fixtures kept in the repo are reference-only and must not be used as the primary acceptance baseline for parser parity in later tasks unless the user explicitly broadens scope again.

## Task 1: Replace The Sample Crate With A Public Client Skeleton

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`
- Create: `src/client.rs`
- Create: `src/error.rs`
- Test: `tests/client_construction.rs`

- [ ] **Step 1: Write the failing public-construction test**

```rust
// tests/client_construction.rs
use ytmusicapi::YtMusic;

#[test]
fn constructs_default_client() {
    let client = YtMusic::new().expect("client should build");
    let debug = format!("{client:?}");
    assert!(debug.contains("YtMusic"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test --test client_construction -v`
Expected: FAIL with unresolved import or missing `YtMusic`

- [ ] **Step 3: Add dependencies and the minimal public client surface**

```toml
# Cargo.toml
[package]
name = "ytmusicapi"
version = "0.1.0"
edition = "2024"

[dependencies]
regex = "1.12"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["sync"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
wiremock = "0.6"
```

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("http client build failed: {0}")]
    HttpClientBuild(#[source] reqwest::Error),
}
```

```rust
// src/client.rs
use reqwest::Client;

use crate::Error;

#[derive(Clone, Debug)]
pub struct YtMusic {
    pub(crate) http_client: Client,
    pub(crate) base_url: String,
    pub(crate) homepage_url: String,
}

#[derive(Clone, Debug, Default)]
pub struct YtMusicBuilder {
    http_client: Option<Client>,
    base_url: Option<String>,
    homepage_url: Option<String>,
}

impl YtMusic {
    pub fn new() -> Result<Self, Error> {
        Self::builder().build()
    }

    pub fn builder() -> YtMusicBuilder {
        YtMusicBuilder::default()
    }
}

impl YtMusicBuilder {
    pub fn http_client(mut self, client: Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn homepage_url(mut self, homepage_url: impl Into<String>) -> Self {
        self.homepage_url = Some(homepage_url.into());
        self
    }

    pub fn build(self) -> Result<YtMusic, Error> {
        let http_client = match self.http_client {
            Some(client) => client,
            None => Client::builder().build().map_err(Error::HttpClientBuild)?,
        };

        Ok(YtMusic {
            http_client,
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://music.youtube.com/youtubei/v1/".to_owned()),
            homepage_url: self
                .homepage_url
                .unwrap_or_else(|| "https://music.youtube.com".to_owned()),
        })
    }
}
```

```rust
// src/lib.rs
mod client;
mod error;

pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test --test client_construction -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs src/client.rs src/error.rs tests/client_construction.rs
git commit -m "feat: add ytmusic client skeleton"
```

## Task 2: Add Typed Search Query Models And Exact Param Encoding

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/error.rs`
- Create: `src/model/mod.rs`
- Create: `src/model/search.rs`
- Create: `src/search/mod.rs`
- Create: `src/search/params.rs`
- Test: `tests/search_params.rs`

- [ ] **Step 1: Write the failing parameter-encoding tests**

```rust
// tests/search_params.rs
use ytmusicapi::{SearchFilter, SearchQuery};

#[test]
fn default_query_omits_params() {
    let query = SearchQuery::new("oasis wonderwall");
    assert_eq!(query.encoded_params().as_deref(), None);
}

#[test]
fn ignore_spelling_sets_default_params() {
    let query = SearchQuery::new("martin stig andersen - deteriation").ignore_spelling();
    assert_eq!(
        query.encoded_params().as_deref(),
        Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D")
    );
}

#[test]
fn filter_encodings_match_upstream() {
    let cases = [
        (
            SearchFilter::Songs,
            "EgWKAQIIAWoMEA4QChADEAQQCRAF",
            "EgWKAQIIAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Videos,
            "EgWKAQIQAWoMEA4QChADEAQQCRAF",
            "EgWKAQIQAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Albums,
            "EgWKAQIYAWoMEA4QChADEAQQCRAF",
            "EgWKAQIYAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Artists,
            "EgWKAQIgAWoMEA4QChADEAQQCRAF",
            "EgWKAQIgAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Playlists,
            "Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D",
            "Eg-KAQwIABAAGAAgACgBMABCAggBagoQBBADEAkQBRAK",
        ),
    ];

    for (filter, expected, expected_ignore) in cases {
        let filtered = SearchQuery::new("hip hop").with_filter(filter);
        assert_eq!(filtered.encoded_params().as_deref(), Some(expected));

        let ignored = SearchQuery::new("hip hop").with_filter(filter).ignore_spelling();
        assert_eq!(ignored.encoded_params().as_deref(), Some(expected_ignore));
    }
}

#[test]
fn blank_query_is_rejected() {
    let result = SearchQuery::new("   ").validate();
    assert!(result.is_err());
}

#[test]
fn zero_limit_is_rejected() {
    let result = SearchQuery::new("abba").with_limit(0).validate();
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test search_params -v`
Expected: FAIL with missing `SearchQuery`, `SearchFilter`, or `encoded_params`

- [ ] **Step 3: Implement typed query models and the exact encoding table**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("http client build failed: {0}")]
    HttpClientBuild(#[source] reqwest::Error),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
}
```

```rust
// src/model/mod.rs
pub mod search;
```

```rust
// src/model/search.rs
use crate::{search::params::encode_search_params, Error};

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
            return Err(Error::InvalidInput("limit must be greater than zero".to_owned()));
        }

        Ok(())
    }

    pub fn encoded_params(&self) -> Option<String> {
        encode_search_params(self.filter, self.ignore_spelling)
    }
}
```

```rust
// src/search/mod.rs
pub mod params;
```

```rust
// src/search/params.rs
use crate::model::search::SearchFilter;

pub fn encode_search_params(filter: Option<SearchFilter>, ignore_spelling: bool) -> Option<String> {
    match (filter, ignore_spelling) {
        (None, false) => None,
        (None, true) => Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D".to_owned()),
        (Some(SearchFilter::Songs), false) => Some("EgWKAQIIAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Songs), true) => Some("EgWKAQIIAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned()),
        (Some(SearchFilter::Videos), false) => Some("EgWKAQIQAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Videos), true) => Some("EgWKAQIQAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned()),
        (Some(SearchFilter::Albums), false) => Some("EgWKAQIYAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Albums), true) => Some("EgWKAQIYAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned()),
        (Some(SearchFilter::Artists), false) => Some("EgWKAQIgAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Artists), true) => Some("EgWKAQIgAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned()),
        (Some(SearchFilter::Playlists), false) => Some("Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D".to_owned()),
        (Some(SearchFilter::Playlists), true) => Some("Eg-KAQwIABAAGAAgACgBMABCAggBagoQBBADEAkQBRAK".to_owned()),
    }
}
```

```rust
// src/lib.rs
mod client;
mod error;
pub mod model;
pub(crate) mod search;

pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::search::{SearchFilter, SearchQuery};
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test search_params -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib.rs src/error.rs src/model/mod.rs src/model/search.rs src/search/mod.rs src/search/params.rs tests/search_params.rs
git commit -m "feat: add typed search query parameters"
```

## Task 3: Add Visitor-Id Bootstrap, Search Transport, And Error Mapping

**Files:**
- Modify: `src/client.rs`
- Modify: `src/error.rs`
- Modify: `src/search/mod.rs`
- Create: `src/search/request.rs`
- Test: `tests/search_transport.rs`

- [ ] **Step 1: Write the failing mock transport tests**

```rust
// tests/search_transport.rs
use std::collections::HashMap;

use serde_json::json;
use wiremock::matchers::{body_partial_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{Error, SearchFilter, SearchQuery, YtMusic};

#[tokio::test]
async fn search_bootstraps_visitor_id_and_posts_search_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({"VISITOR_DATA":"visitor-id-123"})"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(header("x-goog-visitor-id", "visitor-id-123"))
        .and(body_partial_json(json!({
            "query": "hip hop",
            "params": "EgWKAQIIAWoMEA4QChADEAQQCRAF",
            "context": {
                "client": {
                    "clientName": "WEB_REMIX"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "tabbedSearchResultsRenderer": {
                    "tabs": []
                }
            }
        })))
        .mount(&server)
        .await;

    let http_client = reqwest::Client::builder().build().unwrap();
    let client = YtMusic::builder()
        .http_client(http_client)
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let query = SearchQuery::new("hip hop").with_filter(SearchFilter::Songs);
    let result = client.search(query).await.unwrap();
    assert!(result.is_empty());
}

#[tokio::test]
async fn server_status_is_mapped_to_status_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({"VISITOR_DATA":"visitor-id-123"})"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": { "message": "boom" }
        })))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client.search(SearchQuery::new("abba")).await.unwrap_err();
    match error {
        Error::HttpStatus { status, .. } => assert_eq!(status.as_u16(), 500),
        other => panic!("expected HttpStatus error, got {other:?}"),
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --test search_transport -v`
Expected: FAIL with missing async `search` implementation or missing `HttpStatus`

- [ ] **Step 3: Implement lazy visitor-id fetch, request transport, and error variants**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
    #[error("http client build failed: {0}")]
    HttpClientBuild(#[source] reqwest::Error),
    #[error("http transport failed: {0}")]
    HttpTransport(#[source] reqwest::Error),
    #[error("http status {status}: {message}")]
    HttpStatus {
        status: reqwest::StatusCode,
        message: String,
    },
    #[error("failed to decode json response: {0}")]
    JsonDecode(#[source] serde_json::Error),
    #[error("failed to bootstrap visitor id")]
    MissingVisitorId,
    #[error("failed to parse search response: {0}")]
    Parse(String),
}
```

```rust
// src/search/request.rs
use regex::Regex;
use serde_json::{json, Value};

use crate::{model::search::SearchQuery, Error};

const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0";

pub(crate) async fn bootstrap_visitor_id(
    http_client: &reqwest::Client,
    homepage_url: &str,
) -> Result<String, Error> {
    let body = http_client
        .get(homepage_url)
        .header("user-agent", USER_AGENT)
        .send()
        .await
        .map_err(Error::HttpTransport)?
        .text()
        .await
        .map_err(Error::HttpTransport)?;

    let regex = Regex::new(r#"VISITOR_DATA":"([^"]+)""#).expect("valid regex");
    let captures = regex.captures(&body).ok_or(Error::MissingVisitorId)?;
    Ok(captures[1].to_owned())
}

pub(crate) fn build_search_body(query: &SearchQuery) -> Value {
    let mut body = json!({
        "query": query.query,
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": "1.20260429.01.00",
            },
            "user": {},
        }
    });

    if let Some(params) = query.encoded_params() {
        body["params"] = Value::String(params);
    }

    body
}
```

```rust
// src/client.rs
use std::sync::Arc;

use reqwest::Client;
use tokio::sync::OnceCell;

use crate::{
    model::search::{SearchQuery, SearchResult},
    search::request::{bootstrap_visitor_id, build_search_body},
    Error,
};

#[derive(Clone, Debug)]
pub struct YtMusic {
    pub(crate) http_client: Client,
    pub(crate) base_url: String,
    pub(crate) homepage_url: String,
    pub(crate) visitor_id: Arc<OnceCell<String>>,
}

impl YtMusic {
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, Error> {
        query.validate()?;

        let visitor_id = self
            .visitor_id
            .get_or_try_init(|| bootstrap_visitor_id(&self.http_client, &self.homepage_url))
            .await?;

        let response = self
            .http_client
            .post(format!("{}/search?alt=json", self.base_url.trim_end_matches('/')))
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("origin", self.homepage_url.as_str())
            .header("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0")
            .header("x-goog-visitor-id", visitor_id)
            .json(&build_search_body(&query))
            .send()
            .await
            .map_err(Error::HttpTransport)?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.map_err(Error::HttpTransport)?;
            return Err(Error::HttpStatus { status, message: body });
        }

        let body = response.text().await.map_err(Error::HttpTransport)?;
        let _json: serde_json::Value = serde_json::from_str(&body).map_err(Error::JsonDecode)?;
        Ok(Vec::new())
    }
}

impl YtMusicBuilder {
    pub fn build(self) -> Result<YtMusic, Error> {
        let http_client = match self.http_client {
            Some(client) => client,
            None => Client::builder().build().map_err(Error::HttpClientBuild)?,
        };

        Ok(YtMusic {
            http_client,
            base_url: self
                .base_url
                .unwrap_or_else(|| "https://music.youtube.com/youtubei/v1/".to_owned()),
            homepage_url: self
                .homepage_url
                .unwrap_or_else(|| "https://music.youtube.com".to_owned()),
            visitor_id: Arc::new(OnceCell::new()),
        })
    }
}
```

```rust
// src/search/mod.rs
pub mod params;
pub mod request;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test --test search_transport -v`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/client.rs src/error.rs src/search/mod.rs src/search/request.rs tests/search_transport.rs
git commit -m "feat: add search transport and visitor bootstrap"
```

## Task 4: Capture And Commit The Upstream 1.12.0 Search Fixtures

**Files:**
- Create: `devtools/capture_search_fixtures.py`
- Create: `tests/fixtures/search/raw/default_mixed.json`
- Create: `tests/fixtures/search/raw/songs.json`
- Create: `tests/fixtures/search/raw/videos.json`
- Create: `tests/fixtures/search/raw/albums.json`
- Create: `tests/fixtures/search/raw/artists.json`
- Create: `tests/fixtures/search/raw/playlists.json`
- Create: `tests/fixtures/search/expected/default_mixed.json`
- Create: `tests/fixtures/search/expected/songs.json`
- Create: `tests/fixtures/search/expected/videos.json`
- Create: `tests/fixtures/search/expected/albums.json`
- Create: `tests/fixtures/search/expected/artists.json`
- Create: `tests/fixtures/search/expected/playlists.json`

Task 4 acceptance is narrowed as follows:

- `default_mixed`, `albums`, `artists`, and `playlists` must be strong anonymous golden fixtures.
- `songs` and `videos` may remain as reference captures only if anonymous upstream search does not produce stable fixture-quality results in the current environment.
- Do not synthesize `songs` or `videos` expected outputs from non-search endpoints. If anonymous search is unstable, keep those captures clearly tied to real anonymous `search()` behavior or leave them out of later parser acceptance.

- [ ] **Step 1: Add the fixture capture utility**

```python
# devtools/capture_search_fixtures.py
from __future__ import annotations

import json
from pathlib import Path

from ytmusicapi import YTMusic


FIXTURE_CASES = [
    ("default_mixed", "oasis wonderwall", None, False),
    ("songs", "hip hop playlist", "songs", False),
    ("videos", "hip hop playlist", "videos", False),
    ("albums", "eminem relapse", "albums", False),
    ("artists", "armen van buren", "artists", True),
    ("playlists", "classical music", "playlists", False),
]


class RecordingYTMusic(YTMusic):
    def __init__(self) -> None:
        super().__init__()
        self.last_response = None

    def _send_request(self, endpoint, body, additionalParams=""):
        response = super()._send_request(endpoint, body, additionalParams)
        self.last_response = response
        return response


def main() -> None:
    root = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "search"
    raw_dir = root / "raw"
    expected_dir = root / "expected"
    raw_dir.mkdir(parents=True, exist_ok=True)
    expected_dir.mkdir(parents=True, exist_ok=True)

    client = RecordingYTMusic()

    for name, query, filter_name, ignore_spelling in FIXTURE_CASES:
        kwargs = {}
        if filter_name is not None:
            kwargs["filter"] = filter_name
        if ignore_spelling:
            kwargs["ignore_spelling"] = True

        parsed = client.search(query, **kwargs)
        raw = client.last_response

        (raw_dir / f"{name}.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        (expected_dir / f"{name}.json").write_text(json.dumps(parsed, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    main()
```

- [ ] **Step 2: Run the capture utility and verify the fixture files appear**

Run:

```bash
python3 -m venv .venv-fixtures
. .venv-fixtures/bin/activate
pip install "ytmusicapi==1.12.0"
python devtools/capture_search_fixtures.py
find tests/fixtures/search -type f | sort
```

Expected:
- `tests/fixtures/search/raw/*.json` contains six files
- `tests/fixtures/search/expected/*.json` contains six files

- [ ] **Step 3: Commit the baseline fixtures**

```bash
git add devtools/capture_search_fixtures.py tests/fixtures/search
git commit -m "test: add upstream search fixtures"
```

## Task 5: Add Shared Search Models And Parse The Default Mixed Fixture

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/model/mod.rs`
- Modify: `src/model/search.rs`
- Modify: `src/search/mod.rs`
- Create: `src/model/common.rs`
- Create: `src/search/parse.rs`
- Test: `tests/search_parser.rs`

- [ ] **Step 1: Write the failing default-fixture parser test**

```rust
// tests/search_parser.rs
use std::fs;
use std::path::PathBuf;

use serde_json::Value;
use ytmusicapi::{SearchFilter, SearchResult};

fn fixture(path: &str) -> Value {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text = fs::read_to_string(root.join(path)).expect("fixture should exist");
    serde_json::from_str(&text).expect("fixture json should parse")
}

#[test]
fn default_mixed_fixture_matches_expected_top_level_types() {
    let raw = fixture("tests/fixtures/search/raw/default_mixed.json");
    let expected = fixture("tests/fixtures/search/expected/default_mixed.json");

    let parsed = ytmusicapi::internal::parse_search_response_for_test(&raw, None).unwrap();
    let expected_items = expected.as_array().unwrap();

    assert_eq!(parsed.len(), expected_items.len());
    assert!(parsed.iter().any(|item| matches!(item, SearchResult::Song(_))));
    assert!(parsed.iter().any(|item| matches!(item, SearchResult::Video(_))));
    assert!(parsed.iter().any(|item| matches!(item, SearchResult::Album(_))));
    assert!(parsed.iter().any(|item| matches!(item, SearchResult::Artist(_))));
    assert!(parsed.iter().any(|item| matches!(item, SearchResult::Playlist(_))));
}
```

- [ ] **Step 2: Run the parser test to verify it fails**

Run: `cargo test --test search_parser default_mixed_fixture_matches_expected_top_level_types -v`
Expected: FAIL with missing `SearchResult` or missing parser helper

- [ ] **Step 3: Implement shared models, result variants, and the default parser path**

```rust
// src/model/common.rs
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtistRef {
    pub name: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AlbumRef {
    pub name: String,
    pub id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Thumbnail {
    pub url: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
```

```rust
// src/model/search.rs
use serde::{Deserialize, Serialize};

use crate::{model::common::{AlbumRef, ArtistRef, Thumbnail}, search::params::encode_search_params, Error};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SongResult {
    pub category: Option<String>,
    pub video_id: String,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
    pub duration: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct VideoResult {
    pub category: Option<String>,
    pub video_id: String,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    pub duration: Option<String>,
    pub views: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AlbumResult {
    pub category: Option<String>,
    pub browse_id: String,
    pub playlist_id: Option<String>,
    pub title: String,
    pub artists: Vec<ArtistRef>,
    pub album_type: Option<String>,
    pub year: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtistResult {
    pub category: Option<String>,
    pub browse_id: String,
    pub name: String,
    pub subscribers: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlaylistResult {
    pub category: Option<String>,
    pub browse_id: Option<String>,
    pub playlist_id: Option<String>,
    pub title: String,
    pub author: Option<String>,
    pub item_count: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SearchResult {
    Song(SongResult),
    Video(VideoResult),
    Album(AlbumResult),
    Artist(ArtistResult),
    Playlist(PlaylistResult),
}
```

```rust
// src/search/parse.rs
use serde_json::Value;

use crate::{
    model::search::{SearchFilter, SearchResult},
    Error,
};

pub(crate) fn parse_search_response(
    response: &Value,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>, Error> {
    let _ = filter;
    let _ = response;
    Err(Error::Parse("parser not implemented yet".to_owned()))
}
```

```rust
// src/lib.rs
mod client;
mod error;
pub mod model;
pub(crate) mod search;

pub mod internal {
    use serde_json::Value;

    use crate::{
        model::search::{SearchFilter, SearchResult},
        search::parse::parse_search_response,
        Error,
    };

    pub fn parse_search_response_for_test(
        value: &Value,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, Error> {
        parse_search_response(value, filter)
    }
}
```

- [ ] **Step 4: Replace the parser stub with real parsing logic for the default mixed fixture**

Implement the real body of `src/search/parse.rs` so it:

- navigates `contents.tabbedSearchResultsRenderer.tabs[*].tabRenderer.content`
- pulls `musicCardShelfRenderer` top-result payloads when present
- pulls `musicShelfRenderer.contents` result rows
- detects `song`, `video`, `album`, `artist`, and `playlist` rows
- preserves category labels
- maps repeated artists and thumbnails into `ArtistRef` and `Thumbnail`
- returns `Error::Parse` on missing required fields for a supported variant

- [ ] **Step 5: Run the parser test to verify it passes**

Run: `cargo test --test search_parser default_mixed_fixture_matches_expected_top_level_types -v`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/model/mod.rs src/model/common.rs src/model/search.rs src/search/mod.rs src/search/parse.rs tests/search_parser.rs
git commit -m "feat: parse default search results"
```

## Task 6: Finish Filtered Parser Coverage And Wire The Real `search` Method

**Files:**
- Modify: `src/client.rs`
- Modify: `src/error.rs`
- Modify: `src/search/mod.rs`
- Modify: `src/search/parse.rs`
- Modify: `tests/search_parser.rs`

- [ ] **Step 1: Expand the parser tests to cover each supported filtered fixture**

```rust
// add to tests/search_parser.rs
#[test]
fn filtered_fixture_result_types_match_expected_filter() {
    let cases = [
        ("albums", SearchFilter::Albums, "album"),
        ("artists", SearchFilter::Artists, "artist"),
        ("playlists", SearchFilter::Playlists, "playlist"),
    ];

    for (name, filter, expected_type) in cases {
        let raw = fixture(&format!("tests/fixtures/search/raw/{name}.json"));
        let parsed = ytmusicapi::internal::parse_search_response_for_test(&raw, Some(filter)).unwrap();
        assert!(!parsed.is_empty(), "{name} fixture should parse at least one result");

        for item in parsed {
            let actual = match item {
                SearchResult::Song(_) => "song",
                SearchResult::Video(_) => "video",
                SearchResult::Album(_) => "album",
                SearchResult::Artist(_) => "artist",
                SearchResult::Playlist(_) => "playlist",
            };
            assert_eq!(actual, expected_type, "unexpected result type for fixture {name}");
        }
    }
}
```

- [ ] **Step 2: Run the expanded parser tests to verify they fail**

Run: `cargo test --test search_parser -v`
Expected: FAIL because the parser is still incomplete for one or more filtered fixtures

- [ ] **Step 3: Finish the parser and connect the real `search` method to it**

Update `src/client.rs` so the `search` method parses the JSON instead of returning `Vec::new()`:

```rust
let body = response.text().await.map_err(Error::HttpTransport)?;
let json: serde_json::Value = serde_json::from_str(&body).map_err(Error::JsonDecode)?;
crate::search::parse::parse_search_response(&json, query.filter)
```

Update `src/search/parse.rs` so the final parser:

- accepts `filter: Option<SearchFilter>`
- uses the known filter, when provided, to set the result variant without guessing from category text
- keeps default mixed search type inference for `None`
- fills `AlbumResult`, `ArtistResult`, and `PlaylistResult`
- may defer `SongResult` and `VideoResult` fixture-driven acceptance until stable upstream fixture provenance exists
- treats missing required identifiers like `videoId` or `browseId` as `Error::Parse`
- keeps optional upstream data such as `views`, `duration`, `year`, and `subscribers` as `Option<T>`

- [ ] **Step 4: Run the parser, transport, and full crate test suite**

Run:

```bash
cargo test --test search_parser -v
cargo test --test search_transport -v
cargo test -v
```

Expected:
- all parser tests PASS
- transport tests PASS
- full test suite PASS

- [ ] **Step 5: Commit**

```bash
git add src/client.rs src/error.rs src/search/mod.rs src/search/parse.rs tests/search_parser.rs
git commit -m "feat: add typed search parsing"
```

## Task 7: Final Library Polish And Verification

**Files:**
- Modify: `src/lib.rs`
- Modify: any files touched in prior tasks if needed for cleanup only

- [ ] **Step 1: Ensure the crate exports the final first-slice public surface**

`src/lib.rs` must publicly export:

```rust
pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::search::{
    AlbumResult,
    ArtistResult,
    PlaylistResult,
    SearchFilter,
    SearchQuery,
    SearchResult,
    SongResult,
    VideoResult,
};
```

- [ ] **Step 2: Run formatting**

Run: `cargo fmt`
Expected: no output

- [ ] **Step 3: Run linting**

Run: `cargo clippy --all-targets --all-features`
Expected: PASS with no warnings promoted to errors

- [ ] **Step 4: Run the full test suite**

Run: `cargo test`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src tests
git commit -m "chore: polish foundation search slice"
```

## Self-Review

**Spec coverage**
- Async-first client: covered by Tasks 1 and 3.
- Established crates: covered by Task 1 dependency setup.
- Typed `search` request model: covered by Task 2.
- Public-catalog unauthenticated request flow: covered by Task 3.
- Strongly typed top-level results: covered by Tasks 5 and 6.
- Fixture-based `1.12.0` baseline tests: covered by Task 4 and Task 6.
- Explicit unsupported scope: covered by Task 2 validation and Task 5/6 parse discipline.

**Placeholder scan**
- No `TBD`, `TODO`, “implement later”, or “similar to Task N” references remain.
- The Task 5 parser stub is paired immediately with the next implementation step and names the exact replacement error variant and file.

**Type consistency**
- Public API names are fixed in the “Public API Targets” section and reused consistently in every task.
- `SearchFilter`, `SearchQuery`, `SearchResult`, and `Error` variant names are consistent across tasks.
