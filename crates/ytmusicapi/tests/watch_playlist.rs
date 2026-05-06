use serde_json::{Value, json};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use ytmusicapi::{ContinuationToken, Error, WatchPlaylistQuery, YtMusic};

#[test]
fn watch_playlist_query_requires_video_or_playlist_id() {
    let err = WatchPlaylistQuery::new().validate().unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message)
            if message == "watch playlist query requires video_id or playlist_id"
    ));
}

#[test]
fn watch_playlist_query_rejects_shuffle_without_playlist_id() {
    let err = WatchPlaylistQuery::new()
        .with_video_id("video-1")
        .shuffle()
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message)
            if message == "watch playlist shuffle requires playlist_id"
    ));
}

#[test]
fn watch_playlist_query_rejects_radio_and_shuffle_together() {
    let err = WatchPlaylistQuery::new()
        .with_playlist_id("PL123")
        .radio()
        .shuffle()
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message)
            if message == "watch playlist shuffle cannot be combined with radio"
    ));
}

#[test]
fn watch_playlist_query_allows_video_and_playlist_together() {
    let query = WatchPlaylistQuery::new()
        .with_video_id("video-1")
        .with_playlist_id("VLPL123")
        .radio();

    assert!(query.validate().is_ok());
    assert_eq!(query.video_id.as_deref(), Some("video-1"));
    assert_eq!(query.playlist_id.as_deref(), Some("VLPL123"));
    assert!(query.radio);
    assert!(!query.shuffle);
}

#[test]
fn watch_playlist_query_rejects_whitespace_only_ids() {
    let err = WatchPlaylistQuery::new()
        .with_video_id("   ")
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message) if message == "watch playlist video_id must not be blank"
    ));
}

#[test]
fn watch_playlist_query_allows_video_id_only() {
    let query = WatchPlaylistQuery::new().with_video_id("video-1");

    assert!(query.validate().is_ok());
    assert_eq!(query.video_id.as_deref(), Some("video-1"));
    assert_eq!(query.playlist_id, None);
    assert!(!query.radio);
    assert!(!query.shuffle);
}

#[test]
fn watch_playlist_query_allows_playlist_id_only() {
    let query = WatchPlaylistQuery::new().with_playlist_id("PL123");

    assert!(query.validate().is_ok());
    assert_eq!(query.video_id, None);
    assert_eq!(query.playlist_id.as_deref(), Some("PL123"));
    assert!(!query.radio);
    assert!(!query.shuffle);
}

#[test]
fn watch_playlist_query_rejects_blank_video_id_with_valid_playlist_id() {
    let err = WatchPlaylistQuery::new()
        .with_video_id("   ")
        .with_playlist_id("PL123")
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message) if message == "watch playlist video_id must not be blank"
    ));
}

#[test]
fn watch_playlist_query_rejects_blank_playlist_id_with_valid_video_id() {
    let err = WatchPlaylistQuery::new()
        .with_video_id("video-1")
        .with_playlist_id("   ")
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message) if message == "watch playlist playlist_id must not be blank"
    ));
}

#[test]
fn watch_playlist_query_rejects_blank_playlist_id_alone() {
    let err = WatchPlaylistQuery::new()
        .with_playlist_id("   ")
        .validate()
        .unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message) if message == "watch playlist playlist_id must not be blank"
    ));
}

#[test]
fn watch_track_serializes_camel_case_and_omits_empty_fields() {
    let track = ytmusicapi::WatchTrack {
        video_id: "video-1".to_owned(),
        title: "Track Title".to_owned(),
        duration: None,
        thumbnails: Vec::new(),
        artists: Vec::new(),
        album: None,
        like_status: None,
        video_type: Some("MUSIC_VIDEO".to_owned()),
        year: None,
        views: Some("123".to_owned()),
        is_in_library: Some(true),
        counterpart: None,
    };

    let value = serde_json::to_value(track).unwrap();
    assert_eq!(
        value,
        json!({
            "videoId": "video-1",
            "title": "Track Title",
            "videoType": "MUSIC_VIDEO",
            "views": "123",
            "isInLibrary": true
        })
    );
}

#[tokio::test]
async fn get_watch_playlist_posts_next_body_and_returns_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/next"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/watch/raw/first_page.json")),
        )
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let page = client
        .get_watch_playlist(WatchPlaylistQuery::new().with_video_id("video-1"))
        .await
        .unwrap();

    assert_eq!(
        page.continuation,
        Some(ContinuationToken::new("watch-token-1").unwrap())
    );
    assert_eq!(page.items[0].video_id, "video-1");

    let requests = server.received_requests().await.unwrap();
    let request = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: Value = serde_json::from_slice(&request.body).unwrap();

    assert_eq!(request.url.path(), "/youtubei/v1/next");
    assert_eq!(body["videoId"], "video-1");
    assert_eq!(body["playlistId"], "RDAMVMvideo-1");
    assert_eq!(body["enablePersistentPlaylistPanel"], true);
    assert_eq!(body["isAudioOnly"], true);
    assert_eq!(body["tunerSettingValue"], "AUTOMIX_SETTING_NORMAL");
}

#[tokio::test]
async fn get_watch_playlist_continuation_posts_continuation_body_and_returns_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/next"))
        .and(|request: &Request| {
            serde_json::from_slice::<Value>(&request.body)
                .ok()
                .and_then(|body| {
                    body.get("continuation")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                == Some("watch-token-1".to_owned())
        })
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/watch/raw/continuation.json")),
        )
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let page = client
        .get_watch_playlist_continuation(ContinuationToken::new("watch-token-1").unwrap())
        .await
        .unwrap();

    assert_eq!(
        page.continuation,
        Some(ContinuationToken::new("watch-token-2").unwrap())
    );
    assert_eq!(page.items[0].video_id, "video-3");
}

#[tokio::test]
async fn get_watch_playlist_uses_shuffle_and_radio_params() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/next"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/watch/raw/first_page.json")),
        )
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    client
        .get_watch_playlist(
            WatchPlaylistQuery::new()
                .with_playlist_id("VLPL123")
                .shuffle(),
        )
        .await
        .unwrap();
    client
        .get_watch_playlist(WatchPlaylistQuery::new().with_video_id("video-1").radio())
        .await
        .unwrap();

    let requests = server.received_requests().await.unwrap();
    let bodies: Vec<Value> = requests
        .iter()
        .filter(|request| request.method.as_str() == "POST")
        .map(|request| serde_json::from_slice::<Value>(&request.body).unwrap())
        .collect();

    assert!(
        bodies
            .iter()
            .any(|body| body["playlistId"] == "PL123" && body["params"] == "wAEB8gECKAE%3D")
    );
    assert!(
        bodies
            .iter()
            .any(|body| body["videoId"] == "video-1" && body["params"] == "wAEB")
    );
}
