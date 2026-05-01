use std::path::PathBuf;

use ytmusicapi::YtMusic;

#[tokio::test]
#[ignore = "requires local browser.json generated from browser.txt and live network access"]
async fn get_library_playlists_live_smoke_test() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let browser_json = repo_root.join("browser.json");

    assert!(
        browser_json.exists(),
        "run `cargo run -p ytmusicapi-cli < browser.txt` from the repo root first"
    );

    let client = YtMusic::from_browser_auth_file(&browser_json).unwrap();
    let playlists = client.get_library_playlists().await.unwrap();
    assert!(
        !playlists.is_empty(),
        "expected at least one library playlist from the authenticated account"
    );
}
