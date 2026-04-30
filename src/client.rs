use reqwest::Client;

use crate::Error;

#[derive(Clone, Debug)]
#[allow(dead_code)]
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
