use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::Value;
use tempfile::tempdir;
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::capture::CaptureConfig;
use ytmusicapi::{SearchQuery, YtMusic};

static ENV_LOCK: Mutex<()> = Mutex::new(());

struct CaptureEnvGuard {
    dir: Option<OsString>,
    label: Option<OsString>,
}

impl CaptureEnvGuard {
    fn set(dir: Option<&Path>, label: Option<&str>) -> Self {
        let guard = Self {
            dir: env::var_os("YTMUSICAPI_CAPTURE_DIR"),
            label: env::var_os("YTMUSICAPI_CAPTURE_LABEL"),
        };

        unsafe {
            match dir {
                Some(dir) => env::set_var("YTMUSICAPI_CAPTURE_DIR", dir),
                None => env::remove_var("YTMUSICAPI_CAPTURE_DIR"),
            }

            match label {
                Some(label) => env::set_var("YTMUSICAPI_CAPTURE_LABEL", label),
                None => env::remove_var("YTMUSICAPI_CAPTURE_LABEL"),
            }
        }

        guard
    }
}

impl Drop for CaptureEnvGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.dir {
                Some(dir) => env::set_var("YTMUSICAPI_CAPTURE_DIR", dir),
                None => env::remove_var("YTMUSICAPI_CAPTURE_DIR"),
            }

            match &self.label {
                Some(label) => env::set_var("YTMUSICAPI_CAPTURE_LABEL", label),
                None => env::remove_var("YTMUSICAPI_CAPTURE_LABEL"),
            }
        }
    }
}

#[test]
fn capture_is_disabled_when_capture_dir_is_absent() {
    let _env_lock = ENV_LOCK.lock().unwrap();
    let _env = CaptureEnvGuard::set(None, Some("search-audit"));

    assert!(CaptureConfig::from_env().is_none());
}

#[test]
fn capture_is_enabled_when_capture_dir_and_label_are_present() {
    let _env_lock = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let _env = CaptureEnvGuard::set(Some(dir.path()), Some("search-audit"));

    let config = CaptureConfig::from_env().expect("capture config should be enabled");

    assert_eq!(config.label(), "search-audit");
    assert_eq!(
        config.path_for_endpoint("search"),
        dir.path().join("search-audit").join("search.json")
    );
}

#[test]
fn search_transport_writes_raw_fixture_when_capture_is_enabled() {
    let _env_lock = ENV_LOCK.lock().unwrap();
    let dir = tempdir().unwrap();
    let _env = CaptureEnvGuard::set(Some(dir.path()), Some("search-audit"));

    let expected_json: Value =
        serde_json::from_str(include_str!("fixtures/search/raw/default_mixed.json")).unwrap();

    Runtime::new().unwrap().block_on(async {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.01.00" });"#,
            ))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/youtubei/v1/search"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(include_str!("fixtures/search/raw/default_mixed.json")),
            )
            .mount(&server)
            .await;

        let client = YtMusic::builder()
            .homepage_url(server.uri())
            .base_url(format!("{}/youtubei/v1/", server.uri()))
            .build()
            .unwrap();

        let page = client.search(SearchQuery::new("abba")).await.unwrap();
        assert!(!page.items.is_empty());
    });

    let captured_path: PathBuf = dir.path().join("search-audit").join("search.json");
    let captured = fs::read_to_string(captured_path).unwrap();

    assert_eq!(
        captured,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&expected_json).unwrap()
        )
    );
}
