use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Error;

const CAPTURE_DIR_ENV: &str = "YTMUSICAPI_CAPTURE_DIR";
const CAPTURE_LABEL_ENV: &str = "YTMUSICAPI_CAPTURE_LABEL";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureConfig {
    dir: PathBuf,
    label: String,
}

impl CaptureConfig {
    pub fn from_env() -> Option<Self> {
        let dir = env::var_os(CAPTURE_DIR_ENV)?;
        let label = env::var_os(CAPTURE_LABEL_ENV)?;

        Some(Self {
            dir: PathBuf::from(dir),
            label: sanitize_component(&label.to_string_lossy(), "capture"),
        })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn path_for_endpoint(&self, endpoint: &str) -> PathBuf {
        self.dir.join(&self.label).join(format!(
            "{}.json",
            sanitize_component(endpoint.trim_matches('/'), "response")
        ))
    }
}

pub(crate) fn write_raw_response(
    endpoint: &str,
    response: &serde_json::Value,
) -> Result<(), Error> {
    let Some(config) = CaptureConfig::from_env() else {
        return Ok(());
    };

    let path = config.path_for_endpoint(endpoint);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| write_error("create capture directory", parent, error))?;
    }

    let mut pretty = serde_json::to_string_pretty(response).map_err(|error| {
        Error::Parse(format!(
            "failed to serialize captured response for {endpoint}: {error}"
        ))
    })?;
    pretty.push('\n');

    fs::write(&path, pretty).map_err(|error| write_error("write captured response", &path, error))
}

fn sanitize_component(input: &str, fallback: &str) -> String {
    let sanitized = input
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned();

    if sanitized.is_empty() {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn write_error(action: &str, path: &Path, error: std::io::Error) -> Error {
    Error::Parse(format!("failed to {action} at {}: {error}", path.display()))
}
