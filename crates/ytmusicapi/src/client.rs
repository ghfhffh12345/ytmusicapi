use std::{fmt, sync::Arc};

use reqwest::{Client, header};
use tokio::sync::OnceCell;

use crate::{
    Error, SearchFilter, SearchQuery, SearchResult,
    search::{
        parse::{parse_search_continuation_response, parse_search_response},
        request::{
            BootstrapConfig, USER_AGENT, bootstrap_config as fetch_bootstrap_config,
            build_continuation_body, build_library_albums_body, build_library_artists_body,
            build_library_channels_body, build_library_playlists_body, build_library_podcasts_body,
            build_library_songs_body, build_library_subscriptions_body, build_liked_songs_body,
            build_saved_episodes_body, build_search_body,
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

    pub async fn search(&self, query: SearchQuery) -> Result<crate::Page<SearchResult>, Error> {
        query.validate()?;

        let bootstrap_config = self.bootstrap_config().await?;
        if self.browser_auth.is_some() {
            let mut authenticated_search_config = bootstrap_config.clone();
            authenticated_search_config.client_version =
                self.search_client_version(bootstrap_config);
            let authenticated_body = build_search_body(&query, &authenticated_search_config);
            let authenticated_result = self
                .search_with_transport(
                    bootstrap_config,
                    authenticated_body,
                    self.browser_auth.as_ref(),
                    query.filter,
                )
                .await;
            match authenticated_result {
                Ok(results) => Ok(results),
                Err(Error::HttpTransport(_)) | Err(Error::HttpStatus { .. }) => {
                    let anonymous_body = build_search_body(&query, bootstrap_config);
                    self.search_with_transport(bootstrap_config, anonymous_body, None, query.filter)
                        .await
                }
                Err(error) => Err(error),
            }
        } else {
            let anonymous_body = build_search_body(&query, bootstrap_config);
            self.search_with_transport(bootstrap_config, anonymous_body, None, query.filter)
                .await
        }
    }

    pub async fn search_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::SearchResult>, Error> {
        let bootstrap = self.bootstrap_config().await?;

        if self.browser_auth.is_some() {
            let client_version = self.search_client_version(bootstrap);
            let authenticated_body = build_continuation_body(&token, &client_version);
            match self
                .search_with_transport(
                    bootstrap,
                    authenticated_body,
                    self.browser_auth.as_ref(),
                    None,
                )
                .await
            {
                Ok(page) => Ok(page),
                Err(Error::HttpTransport(_)) | Err(Error::HttpStatus { .. }) => {
                    let anonymous_body = build_continuation_body(&token, &bootstrap.client_version);
                    self.search_with_transport(bootstrap, anonymous_body, None, None)
                        .await
                }
                Err(error) => Err(error),
            }
        } else {
            let body = build_continuation_body(&token, &bootstrap.client_version);
            self.search_with_transport(bootstrap, body, None, None)
                .await
        }
    }

    async fn search_with_transport(
        &self,
        bootstrap: &BootstrapConfig,
        body: serde_json::Value,
        browser_auth: Option<&crate::auth::BrowserAuthHeaders>,
        filter: Option<SearchFilter>,
    ) -> Result<crate::Page<SearchResult>, Error> {
        let response_json = self
            .search_raw_transport(bootstrap, body, browser_auth)
            .await?;
        parse_search_page(&response_json, filter)
    }

    fn search_client_version(&self, bootstrap: &BootstrapConfig) -> String {
        self.browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .cloned()
            .unwrap_or_else(|| bootstrap.client_version.clone())
    }

    async fn search_raw_transport(
        &self,
        bootstrap: &BootstrapConfig,
        body: serde_json::Value,
        browser_auth: Option<&crate::auth::BrowserAuthHeaders>,
    ) -> Result<serde_json::Value, Error> {
        let url = format!(
            "{}/search?alt=json&key={}",
            self.base_url.trim_end_matches('/'),
            bootstrap.innertube_api_key
        );
        let request = self.http_client.post(url).body(body.to_string());
        let request = if let Some(browser_auth) = browser_auth {
            request.headers(browser_auth.to_header_map(Some(&bootstrap.visitor_id))?)
        } else {
            request
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::USER_AGENT, USER_AGENT)
                .header("x-goog-visitor-id", &bootstrap.visitor_id)
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
        Ok(response_json)
    }

    pub async fn get_library_playlists(
        &self,
    ) -> Result<crate::Page<crate::LibraryPlaylist>, Error> {
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

    pub async fn get_library_playlists_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibraryPlaylist>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_playlists_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::playlists::parse_library_playlists_continuation(&response)
    }

    pub async fn get_account_info(&self) -> Result<crate::AccountInfo, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_account_info requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": client_version,
                }
            }
        });

        let response = self
            .post_authenticated_json("account/account_menu", body)
            .await?;
        crate::library::account::parse_account_info_response(&response)
    }

    pub async fn get_library_artists(&self) -> Result<crate::Page<crate::LibraryArtist>, Error> {
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

    pub async fn get_library_artists_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibraryArtist>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_artists_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::artists::parse_library_artists_continuation(&response)
    }

    pub async fn get_library_albums(&self) -> Result<crate::Page<crate::LibraryAlbum>, Error> {
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

    pub async fn get_library_albums_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibraryAlbum>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_albums_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::albums::parse_library_albums_continuation(&response)
    }

    pub async fn get_library_subscriptions(
        &self,
    ) -> Result<crate::Page<crate::LibrarySubscription>, Error> {
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

    pub async fn get_library_subscriptions_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibrarySubscription>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_subscriptions_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::subscriptions::parse_library_subscriptions_continuation(&response)
    }

    pub async fn get_library_channels(&self) -> Result<crate::Page<crate::LibraryChannel>, Error> {
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

    pub async fn get_library_channels_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibraryChannel>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_channels_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::channels::parse_library_channels_continuation(&response)
    }

    pub async fn get_library_podcasts(&self) -> Result<crate::Page<crate::LibraryPodcast>, Error> {
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

    pub async fn get_library_podcasts_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibraryPodcast>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_podcasts_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::podcasts::parse_library_podcasts_continuation(&response)
    }

    pub async fn get_library_songs(&self) -> Result<crate::Page<crate::LibrarySong>, Error> {
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

    pub async fn get_library_songs_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::Page<crate::LibrarySong>, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_library_songs_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::songs::parse_library_songs_continuation(&response)
    }

    pub async fn get_liked_songs(&self) -> Result<crate::LikedSongsPage, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_liked_songs requires browser authentication".to_owned(),
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
        let body = build_liked_songs_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::liked_songs::parse_liked_songs_response(&response)
    }

    pub async fn get_liked_songs_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::LikedSongsPage, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_liked_songs_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::liked_songs::parse_liked_songs_continuation(&response)
    }

    pub async fn get_saved_episodes(&self) -> Result<crate::SavedEpisodesPage, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_saved_episodes requires browser authentication".to_owned(),
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
        let body = build_saved_episodes_body(&browse_config);

        let response = self.post_browse(body).await?;
        crate::library::saved_episodes::parse_saved_episodes_response(&response)
    }

    pub async fn get_saved_episodes_continuation(
        &self,
        token: crate::ContinuationToken,
    ) -> Result<crate::SavedEpisodesPage, Error> {
        if self.browser_auth.is_none() {
            return Err(Error::UnsupportedFeature(
                "get_saved_episodes_continuation requires browser authentication".to_owned(),
            ));
        }

        let bootstrap_config = self.bootstrap_config().await?;
        let client_version = self
            .browser_auth
            .as_ref()
            .and_then(|browser_auth| browser_auth.headers.get("x-youtube-client-version"))
            .map(String::as_str)
            .unwrap_or(&bootstrap_config.client_version);
        let body = build_continuation_body(&token, client_version);

        let response = self.post_browse(body).await?;
        crate::library::saved_episodes::parse_saved_episodes_continuation(&response)
    }

    async fn bootstrap_config(&self) -> Result<&BootstrapConfig, Error> {
        self.bootstrap_config
            .get_or_try_init(|| async {
                fetch_bootstrap_config(&self.http_client, &self.homepage_url).await
            })
            .await
    }

    async fn post_browse(&self, body: serde_json::Value) -> Result<serde_json::Value, Error> {
        self.post_authenticated_json("browse", body).await
    }

    async fn post_authenticated_json(
        &self,
        endpoint: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let bootstrap_config = self.bootstrap_config().await?;
        let url = format!(
            "{}/{}?alt=json&key={}",
            self.base_url.trim_end_matches('/'),
            endpoint.trim_start_matches('/'),
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

fn parse_search_page(
    response_json: &serde_json::Value,
    filter: Option<SearchFilter>,
) -> Result<crate::Page<SearchResult>, Error> {
    if response_json
        .pointer("/continuationContents/musicShelfContinuation")
        .is_some()
    {
        parse_search_continuation_response(response_json)
    } else {
        parse_search_response(response_json, filter)
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
    if response_json
        .pointer("/contents/tabbedSearchResultsRenderer")
        .and_then(serde_json::Value::as_object)
        .is_some()
        || response_json
            .pointer("/continuationContents/musicShelfContinuation")
            .and_then(serde_json::Value::as_object)
            .is_some()
    {
        Ok(())
    } else {
        Err(Error::Parse(
            "search response missing contents.tabbedSearchResultsRenderer".to_owned(),
        ))
    }
}
