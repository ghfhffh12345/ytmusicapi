use std::sync::Arc;

use reqwest::{Client, header};
use tokio::sync::OnceCell;

use crate::{
    Error, SearchQuery, SearchResult,
    search::{
        parse::parse_search_response,
        request::{BootstrapConfig, USER_AGENT, bootstrap_config, build_search_body},
    },
};

#[derive(Clone, Debug)]
pub struct YtMusic {
    pub(crate) http_client: Client,
    pub(crate) base_url: String,
    pub(crate) homepage_url: String,
    pub(crate) bootstrap_config: Arc<OnceCell<BootstrapConfig>>,
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

    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn homepage_url(&self) -> &str {
        &self.homepage_url
    }

    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, Error> {
        query.validate()?;

        let bootstrap_config = self
            .bootstrap_config
            .get_or_try_init(|| async {
                bootstrap_config(&self.http_client, &self.homepage_url).await
            })
            .await?;

        let url = format!(
            "{}/search?alt=json&key={}",
            self.base_url.trim_end_matches('/'),
            bootstrap_config.innertube_api_key
        );
        let body = build_search_body(&query, bootstrap_config).to_string();
        let response = self
            .http_client
            .post(url)
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::USER_AGENT, USER_AGENT)
            .header("x-goog-visitor-id", &bootstrap_config.visitor_id)
            .body(body)
            .send()
            .await
            .map_err(Error::HttpTransport)?;

        let status = response.status();
        let response_body = response.text().await.map_err(Error::HttpTransport)?;

        if !status.is_success() {
            let message = extract_status_message(&response_body);
            return Err(Error::HttpStatus { status, message });
        }

        let response_json: serde_json::Value =
            serde_json::from_str(&response_body).map_err(Error::JsonDecode)?;
        validate_search_response_structure(&response_json)?;

        let mut results = parse_search_response(&response_json, query.filter)?;
        results.truncate(query.limit);
        Ok(results)
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
            bootstrap_config: Arc::new(OnceCell::new()),
        })
    }
}

fn extract_status_message(response_body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(response_body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| error.get("message"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| response_body.to_owned())
}

fn validate_search_response_structure(response_json: &serde_json::Value) -> Result<(), Error> {
    match response_json
        .pointer("/contents/tabbedSearchResultsRenderer")
        .and_then(serde_json::Value::as_object)
    {
        Some(_) => Ok(()),
        None => Err(Error::Parse(
            "search response missing contents.tabbedSearchResultsRenderer".to_owned(),
        )),
    }
}
