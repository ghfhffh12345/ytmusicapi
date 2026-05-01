use std::fs;

use assert_cmd::Command;
use tempfile::tempdir;

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
Cookie: __Secure-3PAPISID=test-sapisid\n"
}

#[test]
fn writes_browser_json_in_current_directory() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("ytmusicapi-cli")
        .unwrap()
        .current_dir(dir.path())
        .write_stdin(firefox_headers())
        .assert()
        .success();

    let output = fs::read_to_string(dir.path().join("browser.json")).unwrap();
    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(value["x-goog-authuser"], "0");
}

#[test]
fn rejects_incomplete_header_input() {
    let dir = tempdir().unwrap();

    Command::cargo_bin("ytmusicapi-cli")
        .unwrap()
        .current_dir(dir.path())
        .write_stdin("Cookie: __Secure-3PAPISID=test-sapisid\n")
        .assert()
        .failure();

    assert!(!dir.path().join("browser.json").exists());
}
