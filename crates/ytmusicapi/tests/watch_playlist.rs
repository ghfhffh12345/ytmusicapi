use serde_json::json;
use ytmusicapi::{Error, WatchPlaylistQuery};

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
        Error::InvalidInput(message)
            if message == "watch playlist query requires video_id or playlist_id"
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
