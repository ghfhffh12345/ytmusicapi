use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("invalid input: {0}")]
    InvalidInput(String),
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
    #[error("failed to bootstrap anonymous client config: missing {0}")]
    MissingBootstrapField(&'static str),
    #[error("failed to parse search response: {0}")]
    Parse(String),
    #[error("unsupported feature: {0}")]
    UnsupportedFeature(String),
}
