use std::{fmt, sync::Arc};

use reqwest::{Client, header};
use tokio::sync::OnceCell;

use crate::{
    Error, SearchQuery, SearchResult,
    search::{
        parse::parse_search_response,
        request::{
            BootstrapConfig, USER_AGENT, bootstrap_config as fetch_bootstrap_config,
            build_library_albums_body, build_library_artists_body, build_library_channels_body,
            build_library_playlists_body, build_library_podcasts_body, build_library_songs_body,
            build_library_subscriptions_body, build_search_body,
        },
    },
};

#[derive(Clone)]
pub struct YtMusic {
    pub(crate) http_client: Client,
    pub(crate) base_url: String,
    pub(crate) homepage_url: String,
    pub(crate) bootstrap_config: Arc<OnceCell<BootstrapConfig>>,
    pub(crate) browser_auth: Option<crate::auth::BrowserAuthHeaders>,
}

#[derive(Clone, Debug, Default)]
pub struct YtMusicBuilder {
    http_client: Option<Client>,
    base_url: Option<String>,
    homepage_url: Option<String>,
    browser_auth_path: Option<std::path::PathBuf>,
}

impl YtMusic {
    pub fn new() -> Result<Self, Error> {
        Self::builder().build()
    }

    pub fn builder() -> YtMusicBuilder {
        YtMusicBuilder::default()
    }

    pub fn from_browser_auth_file(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Self::builder()
            .browser_auth_path(path.as_ref().to_path_buf())
            .build()
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

        let bootstrap_config = self.bootstrap_config().await?;
        if self.browser_auth.is_some() {
            match self
                .search_with_transport(&query, bootstrap_config, true)
                .await
            {
                Ok(results) => Ok(results),
                Err(Error::HttpTransport(_)) | Err(Error::HttpStatus { .. }) => {
                    self.search_with_transport(&query, bootstrap_config, false)
                        .await
                }
                Err(error) => Err(error),
            }
        } else {
            self.search_with_transport(&query, bootstrap_config, false)
                .await
        }
    }

    async fn search_with_transport(
        &self,
        query: &SearchQuery,
        bootstrap_config: &BootstrapConfig,
        authenticated: bool,
    ) -> Result<Vec<SearchResult>, Error> {
        let client_version = if authenticated {
            self.browser_auth
                .as_ref()
                .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
                .map(String::as_str)
                .unwrap_or(&bootstrap_config.client_version)
        } else {
            &bootstrap_config.client_version
        };
        let mut search_config = bootstrap_config.clone();
        search_config.client_version = client_version.to_owned();

        let url = format!(
            "{}/search?alt=json&key={}",
            self.base_url.trim_end_matches('/'),
            bootstrap_config.innertube_api_key
        );
        let body = build_search_body(query, &search_config).to_string();
        let request = self.http_client.post(url).body(body);
        let request = if authenticated {
            let browser_auth = self.browser_auth.as_ref().unwrap();
            request.headers(browser_auth.to_header_map(Some(&bootstrap_config.visitor_id))?)
        } else {
            request
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::USER_AGENT, USER_AGENT)
                .header("x-goog-visitor-id", &bootstrap_config.visitor_id)
        };
        let response = request.send().await.map_err(Error::HttpTransport)?;

        let status = response.status();
        let response_body = response.text().await.map_err(Error::HttpTransport)?;

        if !status.is_success() {
            let message = extract_status_message(&response_body);
            return Err(Error::HttpStatus { status, message });
        }

        let response_json: serde_json::Value =
            serde_json::from_str(&response_body).map_err(Error::JsonDecode)?;
        validate_search_response_structure(&response_json)?;

        let results = parse_search_response(&response_json, query.filter)?;
        Ok(results)
    }

    pub async fn get_library_playlists(&self) -> Result<Vec<crate::LibraryPlaylist>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_playlists requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_playlists_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::playlists::parse_library_playlists_response(&response)
    }

    pub async fn get_library_artists(&self) -> Result<Vec<crate::LibraryArtist>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_artists requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_artists_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::artists::parse_library_artists_response(&response)
    }

    pub async fn get_library_albums(&self) -> Result<Vec<crate::LibraryAlbum>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_albums requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_albums_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::albums::parse_library_albums_response(&response)
    }

    pub async fn get_library_subscriptions(
        &self,
    ) -> Result<Vec<crate::LibrarySubscription>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_subscriptions requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_subscriptions_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::subscriptions::parse_library_subscriptions_response(&response)
    }

    pub async fn get_library_channels(&self) -> Result<Vec<crate::LibraryChannel>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_channels requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_channels_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::channels::parse_library_channels_response(&response)
    }

    pub async fn get_library_podcasts(&self) -> Result<Vec<crate::LibraryPodcast>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_podcasts requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_podcasts_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::podcasts::parse_library_podcasts_response(&response)
    }

    pub async fn get_library_songs(&self) -> Result<Vec<crate::LibrarySong>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_songs requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let mut browse_config = bootstrap_config.clone();
        browse_config.client_version = client_version.to_owned();
        let body = build_library_songs_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::songs::parse_library_songs_response(&response)
    }

    async fn bootstrap_config(&self) -> Result<&BootstrapConfig, Error> {
        self.bootstrap_config
            .get_or_try_init(|| async {
                fetch_bootstrap_config(&self.http_client, &self.homepage_url).await
            })
            .await
    }

    async fn post_browse(&self, body: serde_json::Value) -> Result<serde_json::Value, Error> {
        let bootstrap_config = self.bootstrap_config().await?;
        let url = format!(
            "{}/browse?alt=json&key={}",
            self.base_url.trim_end_matches('/'),
            bootstrap_config.innertube_api_key
        );

        let request = self.http_client.post(url).body(body.to_string());
        let request = if let Some(browser_auth) = &self.browser_auth {
            request.headers(browser_auth.to_header_map(Some(&bootstrap_config.visitor_id))?)
        } else {
            request
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::USER_AGENT, USER_AGENT)
                .header("x-goog-visitor-id", &bootstrap_config.visitor_id)
        };

        let response = request.send().await.map_err(Error::HttpTransport)?;
        let status = response.status();
        let response_body = response.text().await.map_err(Error::HttpTransport)?;

        if !status.is_success() {
            let message = extract_status_message(&response_body);
            return Err(Error::HttpStatus { status, message });
        }

        serde_json::from_str(&response_body).map_err(Error::JsonDecode)
    }
}

impl fmt::Debug for YtMusic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let browser_auth = self.browser_auth.as_ref().map(|_| "<redacted>");

        f.debug_struct("YtMusic")
            .field("http_client", &self.http_client)
            .field("base_url", &self.base_url)
            .field("homepage_url", &self.homepage_url)
            .field("bootstrap_config", &self.bootstrap_config)
            .field("browser_auth", &browser_auth)
            .finish()
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

    pub fn browser_auth_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.browser_auth_path = Some(path.into());
        self
    }

    pub fn build(self) -> Result<YtMusic, Error> {
        let browser_auth = match self.browser_auth_path {
            Some(path) => Some(crate::auth::load_browser_auth_file(&path)?),
            None => None,
        };
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
            browser_auth,
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
