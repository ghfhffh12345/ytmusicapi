use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::Error;

const CAPTURE_DIR_ENV: &str = "YTMUSICAPI_CAPTURE_DIR";
const CAPTURE_LABEL_ENV: &str = "YTMUSICAPI_CAPTURE_LABEL";

#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureConfig {
    dir: PathBuf,
    label: String,
}

impl CaptureConfig {
    fn from_env() -> Option<Self> {
        let dir = env::var_os(CAPTURE_DIR_ENV)?;
        let label = env::var_os(CAPTURE_LABEL_ENV)?;

        Some(Self {
            dir: PathBuf::from(dir),
            label: sanitize_component(&label.to_string_lossy(), "capture"),
        })
    }

    fn path_for_endpoint(&self, endpoint: &str) -> PathBuf {
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

    if sanitized.is_empty() || matches!(sanitized.as_str(), "." | "..") {
        fallback.to_owned()
    } else {
        sanitized
    }
}

fn write_error(action: &str, path: &Path, error: std::io::Error) -> Error {
    Error::Parse(format!("failed to {action} at {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    use tempfile::tempdir;

    use super::CaptureConfig;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct CaptureEnvGuard {
        dir: Option<OsString>,
        label: Option<OsString>,
    }

    impl CaptureEnvGuard {
        fn set(dir: Option<&Path>, label: Option<&str>) -> Self {
            let guard = Self {
                dir: std::env::var_os(super::CAPTURE_DIR_ENV),
                label: std::env::var_os(super::CAPTURE_LABEL_ENV),
            };

            unsafe {
                match dir {
                    Some(dir) => std::env::set_var(super::CAPTURE_DIR_ENV, dir),
                    None => std::env::remove_var(super::CAPTURE_DIR_ENV),
                }

                match label {
                    Some(label) => std::env::set_var(super::CAPTURE_LABEL_ENV, label),
                    None => std::env::remove_var(super::CAPTURE_LABEL_ENV),
                }
            }

            guard
        }
    }

    impl Drop for CaptureEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.dir {
                    Some(dir) => std::env::set_var(super::CAPTURE_DIR_ENV, dir),
                    None => std::env::remove_var(super::CAPTURE_DIR_ENV),
                }

                match &self.label {
                    Some(label) => std::env::set_var(super::CAPTURE_LABEL_ENV, label),
                    None => std::env::remove_var(super::CAPTURE_LABEL_ENV),
                }
            }
        }
    }

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn capture_config_is_disabled_when_capture_dir_is_absent() {
        let _env_lock = lock_env();
        let _env = CaptureEnvGuard::set(None, Some("search-audit"));

        assert!(CaptureConfig::from_env().is_none());
    }

    #[test]
    fn capture_config_is_enabled_when_capture_dir_and_label_are_present() {
        let _env_lock = lock_env();
        let dir = tempdir().unwrap();
        let _env = CaptureEnvGuard::set(Some(dir.path()), Some("search-audit"));

        let config = CaptureConfig::from_env().expect("capture config should be enabled");

        assert_eq!(config.label, "search-audit");
        assert_eq!(
            config.path_for_endpoint("search"),
            dir.path().join("search-audit").join("search.json")
        );
    }

    #[test]
    fn capture_config_normalizes_parent_directory_labels_safely() {
        let _env_lock = lock_env();
        let dir = tempdir().unwrap();
        let _env = CaptureEnvGuard::set(Some(dir.path()), Some(".."));

        let config = CaptureConfig::from_env().expect("capture config should be enabled");

        assert_eq!(config.label, "capture");
        assert_eq!(
            config.path_for_endpoint("search"),
            dir.path().join("capture").join("search.json")
        );
    }
}
