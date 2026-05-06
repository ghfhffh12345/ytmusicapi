use std::fs;

use serde_json::{Value, json};
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use ytmusicapi::{ContinuationToken, Error, WatchPlaylistQuery, YtMusic, setup_browser_auth};

fn browser_auth_json() -> String {
    setup_browser_auth(
        "POST /youtubei/v1/next HTTP/3\n\
Host: music.youtube.com\n\
User-Agent: Mozilla/5.0\n\
Accept: */*\n\
Content-Type: application/json\n\
X-Goog-AuthUser: 0\n\
X-Origin: https://music.youtube.com\n\
X-Youtube-Client-Name: 67\n\
X-Youtube-Client-Version: 1.20250502.01.00\n\
Cookie: __Secure-3PAPISID=test-sapisid\n",
    )
    .unwrap()
}

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

    let dir = tempdir().unwrap();
    let browser_json = dir.path().join("browser.json");
    fs::write(&browser_json, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .browser_auth_path(&browser_json)
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
    assert_eq!(request.url.query(), Some("alt=json&key=test-api-key"));
    assert!(
        request
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.starts_with("SAPISIDHASH "))
            .unwrap_or(false)
    );
    assert_eq!(
        request
            .headers
            .get("cookie")
            .and_then(|value| value.to_str().ok()),
        Some("__Secure-3PAPISID=test-sapisid")
    );
    assert_eq!(
        request
            .headers
            .get("x-goog-authuser")
            .and_then(|value| value.to_str().ok()),
        Some("0")
    );
    assert_eq!(
        request
            .headers
            .get("x-goog-visitor-id")
            .and_then(|value| value.to_str().ok()),
        Some("visitor-id-123")
    );
    assert_eq!(body["videoId"], "video-1");
    assert_eq!(body["playlistId"], "RDAMVMvideo-1");
    assert_eq!(body["enablePersistentPlaylistPanel"], true);
    assert_eq!(body["isAudioOnly"], true);
    assert_eq!(body["tunerSettingValue"], "AUTOMIX_SETTING_NORMAL");
    assert_eq!(
        body["context"]["client"]["clientVersion"],
        "1.20250502.01.00"
    );
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

    let requests = server.received_requests().await.unwrap();
    let request = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: Value = serde_json::from_slice(&request.body).unwrap();

    assert_eq!(request.url.path(), "/youtubei/v1/next");
    assert_eq!(request.url.query(), Some("alt=json&key=test-api-key"));
    assert_eq!(
        request
            .headers
            .get("x-goog-visitor-id")
            .and_then(|value| value.to_str().ok()),
        Some("visitor-id-123")
    );
    assert_eq!(
        body["context"]["client"]["clientVersion"],
        "1.20250501.03.00"
    );
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
        .respond_with(|request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            match body.get("params").and_then(Value::as_str) {
                Some("wAEB8gECKAE%3D") => ResponseTemplate::new(200)
                    .set_body_string(include_str!("fixtures/watch/raw/shuffle_first_page.json")),
                Some("wAEB") => ResponseTemplate::new(200)
                    .set_body_string(include_str!("fixtures/watch/raw/radio_first_page.json")),
                other => panic!("unexpected watch params: {other:?}"),
            }
        })
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let shuffle_page = client
        .get_watch_playlist(
            WatchPlaylistQuery::new()
                .with_playlist_id("VLPL123")
                .shuffle(),
        )
        .await
        .unwrap();
    let radio_page = client
        .get_watch_playlist(WatchPlaylistQuery::new().with_video_id("video-1").radio())
        .await
        .unwrap();

    assert_eq!(shuffle_page.items[0].video_id, "shuffle-video-1");
    assert_eq!(
        shuffle_page.continuation,
        Some(ContinuationToken::new("shuffle-watch-token-1").unwrap())
    );
    assert_eq!(radio_page.items[0].video_id, "radio-video-1");
    assert_eq!(
        radio_page.continuation,
        Some(ContinuationToken::new("radio-watch-token-1").unwrap())
    );

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
