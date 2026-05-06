use serde_json::{Value, json};

use crate::{Error, WatchPlaylistQuery, search::request::BootstrapConfig};

#[allow(dead_code)]
pub(crate) fn build_watch_playlist_body(
    query: &WatchPlaylistQuery,
    bootstrap: &BootstrapConfig,
) -> Result<Value, Error> {
    query.validate()?;

    let mut body = json!({
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": bootstrap.client_version,
            }
        },
        "enablePersistentPlaylistPanel": true,
        "isAudioOnly": true,
        "tunerSettingValue": "AUTOMIX_SETTING_NORMAL",
    });

    if let Some(video_id) = query.video_id.as_deref() {
        body["videoId"] = Value::String(video_id.to_owned());
    }

    let playlist_id = query
        .playlist_id
        .as_deref()
        .map(normalize_playlist_id)
        .map(str::to_owned)
        .or_else(|| {
            query
                .video_id
                .as_deref()
                .map(|video_id| format!("RDAMVM{video_id}"))
        });

    if let Some(playlist_id) = playlist_id {
        body["playlistId"] = Value::String(playlist_id);
    }

    if query.shuffle {
        body["params"] = Value::String("wAEB8gECKAE%3D".to_owned());
    } else if query.radio {
        body["params"] = Value::String("wAEB".to_owned());
    } else if query.video_id.is_some() {
        body["watchEndpointMusicSupportedConfigs"] = json!({
            "watchEndpointMusicConfig": {
                "hasPersistentPlaylistPanel": true,
                "musicVideoType": "MUSIC_VIDEO_TYPE_ATV",
            }
        });
    }

    Ok(body)
}

#[allow(dead_code)]
fn normalize_playlist_id(playlist_id: &str) -> &str {
    playlist_id.strip_prefix("VL").unwrap_or(playlist_id)
}

#[test]
fn build_watch_playlist_body_synthesizes_rdamvm_when_only_video_id_is_present() {
    let bootstrap = BootstrapConfig {
        visitor_id: "visitor-id-123".to_owned(),
        innertube_api_key: "test-api-key".to_owned(),
        client_version: "1.20250501.03.00".to_owned(),
    };

    let body = build_watch_playlist_body(
        &WatchPlaylistQuery::new().with_video_id("video-1"),
        &bootstrap,
    )
    .unwrap();

    assert_eq!(body["videoId"], "video-1");
    assert_eq!(body["playlistId"], "RDAMVMvideo-1");
    assert_eq!(body["enablePersistentPlaylistPanel"], true);
    assert_eq!(body["isAudioOnly"], true);
    assert_eq!(body["tunerSettingValue"], "AUTOMIX_SETTING_NORMAL");
    assert_eq!(
        body["watchEndpointMusicSupportedConfigs"],
        json!({
            "watchEndpointMusicConfig": {
                "hasPersistentPlaylistPanel": true,
                "musicVideoType": "MUSIC_VIDEO_TYPE_ATV",
            }
        })
    );
}

#[test]
fn build_watch_playlist_body_strips_vl_prefix_and_sets_shuffle_params() {
    let bootstrap = BootstrapConfig {
        visitor_id: "visitor-id-123".to_owned(),
        innertube_api_key: "test-api-key".to_owned(),
        client_version: "1.20250501.03.00".to_owned(),
    };

    let body = build_watch_playlist_body(
        &WatchPlaylistQuery::new().with_playlist_id("VLPL123").shuffle(),
        &bootstrap,
    )
    .unwrap();

    assert_eq!(body["playlistId"], "PL123");
    assert_eq!(body["params"], "wAEB8gECKAE%3D");
    assert!(body.get("watchEndpointMusicSupportedConfigs").is_none());
}

#[test]
fn build_watch_playlist_body_sets_radio_params_without_persistent_playlist_config() {
    let bootstrap = BootstrapConfig {
        visitor_id: "visitor-id-123".to_owned(),
        innertube_api_key: "test-api-key".to_owned(),
        client_version: "1.20250501.03.00".to_owned(),
    };

    let body = build_watch_playlist_body(
        &WatchPlaylistQuery::new().with_video_id("video-1").radio(),
        &bootstrap,
    )
    .unwrap();

    assert_eq!(body["params"], "wAEB");
    assert!(body.get("watchEndpointMusicSupportedConfigs").is_none());
}
