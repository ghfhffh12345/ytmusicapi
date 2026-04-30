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
