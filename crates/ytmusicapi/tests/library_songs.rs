use ytmusicapi::{Error, YtMusic};

#[tokio::test]
async fn get_library_songs_requires_browser_auth() {
    let client = YtMusic::new().unwrap();
    let error = client.get_library_songs().await.unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_library_songs requires browser authentication"
    ));
}
