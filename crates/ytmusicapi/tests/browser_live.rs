use std::path::PathBuf;

use ytmusicapi::{SearchFilter, SearchQuery, YtMusic};

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

    let artists = client.get_library_artists().await.unwrap();
    if artists.is_empty() {
        eprintln!(
            "library artists returned 0 items for this account; verified empty-state parsing"
        );
    }

    let albums = client.get_library_albums().await.unwrap();
    if albums.is_empty() {
        eprintln!("library albums returned 0 items for this account; verified empty-state parsing");
    }

    let songs = client
        .search(SearchQuery::new("abba").with_filter(SearchFilter::Songs))
        .await
        .unwrap();
    assert!(
        !songs.is_empty(),
        "expected authenticated filtered songs results for query `abba`"
    );
    assert!(
        songs
            .iter()
            .all(|result| matches!(result, ytmusicapi::SearchResult::Song(_))),
        "expected filtered songs search to return only song results"
    );

    let videos = client
        .search(SearchQuery::new("abba").with_filter(SearchFilter::Videos))
        .await
        .unwrap();
    assert!(
        !videos.is_empty(),
        "expected authenticated filtered videos results for query `abba`"
    );
    assert!(
        videos
            .iter()
            .all(|result| matches!(result, ytmusicapi::SearchResult::Video(_))),
        "expected filtered videos search to return only video results"
    );
}
