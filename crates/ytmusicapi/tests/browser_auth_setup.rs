use std::{fs, path::PathBuf};

use tempfile::tempdir;
use ytmusicapi::{Error, YtMusic, setup_browser_auth};

fn firefox_headers() -> &'static str {
    "POST /youtubei/v1/browse HTTP/3\n\
Host: music.youtube.com\n\
User-Agent: Mozilla/5.0\n\
Accept: */*\n\
Content-Type: application/json\n\
X-Goog-AuthUser: 0\n\
X-Origin: https://music.youtube.com\n\
X-Youtube-Client-Name: 67\n\
X-Youtube-Client-Version: 1.20250501.01.00\n\
Cookie: __Secure-3PAPISID=test-sapisid; VISITOR_PRIVACY_METADATA=CgJVUxIEGgAgVg%3D%3D\n"
}

fn mixed_case_browser_json() -> &'static str {
    r#"{
  "Cookie": "__Secure-3PAPISID=test-sapisid; VISITOR_PRIVACY_METADATA=CgJVUxIEGgAgVg%3D%3D",
  "X-Goog-AuthUser": "0",
  "X-Origin": "https://music.youtube.com",
  "X-Youtube-Client-Name": "67",
  "X-Youtube-Client-Version": "1.20250501.01.00"
}"#
}

#[test]
fn setup_browser_auth_normalizes_firefox_headers() {
    let json = setup_browser_auth(firefox_headers()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(
        value["cookie"],
        "__Secure-3PAPISID=test-sapisid; VISITOR_PRIVACY_METADATA=CgJVUxIEGgAgVg%3D%3D"
    );
    assert_eq!(value["x-goog-authuser"], "0");
    assert_eq!(value["x-origin"], "https://music.youtube.com");
    assert_eq!(value["x-youtube-client-name"], "67");
    assert_eq!(value["x-youtube-client-version"], "1.20250501.01.00");
    assert_eq!(value["origin"], "https://music.youtube.com");
    assert!(value.get("host").is_none());
}

#[test]
fn setup_browser_auth_rejects_missing_required_headers() {
    let error = setup_browser_auth("Cookie: __Secure-3PAPISID=test-sapisid\n").unwrap_err();
    assert!(matches!(error, Error::AuthValidation(_)));
}

#[test]
fn setup_browser_auth_rejects_duplicate_header_names_after_normalization() {
    let error = setup_browser_auth(
        "Cookie: __Secure-3PAPISID=test-sapisid\ncookie: duplicate\nX-Goog-AuthUser: 0\n",
    )
    .unwrap_err();
    assert!(matches!(error, Error::AuthValidation(_)));
}

#[test]
fn authenticated_client_loads_browser_json_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, setup_browser_auth(firefox_headers()).unwrap()).unwrap();

    let client = YtMusic::from_browser_auth_file(&path).unwrap();
    let debug = format!("{client:?}");
    assert!(debug.contains("YtMusic"));
}

#[test]
fn authenticated_client_debug_output_redacts_browser_auth() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, setup_browser_auth(firefox_headers()).unwrap()).unwrap();

    let client = YtMusic::from_browser_auth_file(&path).unwrap();
    let debug = format!("{client:?}");
    assert!(!debug.contains("__Secure-3PAPISID=test-sapisid"));
    assert!(!debug.contains("VISITOR_PRIVACY_METADATA=CgJVUxIEGgAgVg%3D%3D"));
}

#[test]
fn authenticated_client_loads_mixed_case_browser_json_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, mixed_case_browser_json()).unwrap();

    let client = YtMusic::from_browser_auth_file(&path).unwrap();
    let debug = format!("{client:?}");
    assert!(debug.contains("YtMusic"));
}

#[test]
fn authenticated_client_rejects_case_colliding_browser_json_keys() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(
        &path,
        r#"{
  "Cookie": "__Secure-3PAPISID=test-sapisid",
  "cookie": "duplicate",
  "X-Goog-AuthUser": "0"
}"#,
    )
    .unwrap();

    let error = YtMusic::from_browser_auth_file(&path).unwrap_err();
    assert!(matches!(error, Error::AuthValidation(_)));
}

#[test]
fn authenticated_client_reports_missing_file() {
    let missing = PathBuf::from("does-not-exist-browser.json");
    let error = YtMusic::from_browser_auth_file(&missing).unwrap_err();
    assert!(matches!(error, Error::AuthFileRead { .. }));
}
