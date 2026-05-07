use serde_json::Value;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    Error, GetSongResponse, SongByteRange, SongColorInfo, SongMicroformat, SongPlayabilityStatus,
    SongStreamFormat, SongStreamingData, SongVideoDetails, Thumbnail, YtMusic,
};

#[test]
fn get_song_response_serializes_camel_case_and_omits_optional_fields() {
    let response = GetSongResponse {
        video_details: SongVideoDetails {
            video_id: "video-1".to_owned(),
            title: "Track Title".to_owned(),
            length_seconds: 245,
            channel_id: "UC123".to_owned(),
            author: "Artist".to_owned(),
            thumbnails: vec![Thumbnail {
                url: "https://example.com/thumb.jpg".to_owned(),
                width: 60,
                height: 60,
            }],
            allow_ratings: true,
            view_count: "42".to_owned(),
            is_owner_viewing: false,
            is_crawlable: true,
            is_private: false,
            is_unplugged_corpus: false,
            is_live_content: false,
            is_tvfilm_video: false,
            music_video_type: Some("MUSIC_VIDEO_TYPE_ATV".to_owned()),
        },
        playability_status: SongPlayabilityStatus {
            status: "OK".to_owned(),
            playable_in_embed: true,
            context_params: Some("Q0FFU0FnZ0I=".to_owned()),
            audio_only_availability: Some("FEATURE_AVAILABILITY_ALLOWED".to_owned()),
            playback_mode: Some("PLAYBACK_MODE_ALLOW".to_owned()),
        },
        streaming_data: SongStreamingData {
            expires_in_seconds: 21_540,
            server_abr_streaming_url: None,
            formats: vec![SongStreamFormat {
                itag: 18,
                mime_type: "video/mp4; codecs=\"avc1.42001E, mp4a.40.2\"".to_owned(),
                bitrate: 195_233,
                average_bitrate: None,
                content_length: None,
                last_modified: Some("1762374086664772".to_owned()),
                quality: Some("medium".to_owned()),
                quality_label: Some("360p".to_owned()),
                quality_ordinal: Some("QUALITY_ORDINAL_360P".to_owned()),
                projection_type: Some("RECTANGULAR".to_owned()),
                width: Some(360),
                height: Some(360),
                fps: Some(25),
                color_info: None,
                audio_quality: Some("AUDIO_QUALITY_LOW".to_owned()),
                audio_sample_rate: Some(44_100),
                audio_channels: Some(2),
                loudness_db: None,
                track_absolute_loudness_lkfs: None,
                approx_duration_ms: Some(267_331),
                high_replication: None,
                xtags: None,
                init_range: None,
                index_range: None,
                signature_cipher: "s=abc&sp=sig&url=https%3A%2F%2Fexample.com".to_owned(),
            }],
            adaptive_formats: vec![SongStreamFormat {
                itag: 140,
                mime_type: "audio/mp4; codecs=\"mp4a.40.2\"".to_owned(),
                bitrate: 131_062,
                average_bitrate: Some(129_553),
                content_length: Some(4_328_604),
                last_modified: Some("1723441920616650".to_owned()),
                quality: Some("tiny".to_owned()),
                quality_label: None,
                quality_ordinal: Some("QUALITY_ORDINAL_UNKNOWN".to_owned()),
                projection_type: Some("RECTANGULAR".to_owned()),
                width: None,
                height: None,
                fps: None,
                color_info: Some(SongColorInfo {
                    primaries: Some("COLOR_PRIMARIES_BT709".to_owned()),
                    transfer_characteristics: Some(
                        "COLOR_TRANSFER_CHARACTERISTICS_BT709".to_owned(),
                    ),
                    matrix_coefficients: Some("COLOR_MATRIX_COEFFICIENTS_BT709".to_owned()),
                }),
                audio_quality: Some("AUDIO_QUALITY_MEDIUM".to_owned()),
                audio_sample_rate: Some(44_100),
                audio_channels: Some(2),
                loudness_db: Some(-1.68),
                track_absolute_loudness_lkfs: Some(-8.69),
                approx_duration_ms: Some(267_293),
                high_replication: Some(true),
                xtags: None,
                init_range: Some(SongByteRange {
                    start: "0".to_owned(),
                    end: "758".to_owned(),
                }),
                index_range: Some(SongByteRange {
                    start: "759".to_owned(),
                    end: "1114".to_owned(),
                }),
                signature_cipher: "s=def&sp=sig&url=https%3A%2F%2Fexample.com".to_owned(),
            }],
        },
        microformat: Some(SongMicroformat {
            url_canonical: Some("https://music.youtube.com/watch?v=video-1".to_owned()),
            description: Some("Artist".to_owned()),
            category: Some("Music".to_owned()),
            publish_date: Some("2024-08-27T03:04:24-07:00".to_owned()),
            upload_date: Some("2024-08-27T03:04:24-07:00".to_owned()),
            view_count: Some("42".to_owned()),
            available_countries: vec!["KR".to_owned(), "US".to_owned()],
            tags: vec!["tag1".to_owned()],
            noindex: Some(false),
            unlisted: Some(false),
            family_safe: Some(true),
        }),
    };

    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["videoDetails"]["videoId"], "video-1");
    assert_eq!(
        value["streamingData"]["formats"][0]["signatureCipher"],
        "s=abc&sp=sig&url=https%3A%2F%2Fexample.com"
    );
    assert!(
        value["streamingData"]
            .get("serverAbrStreamingUrl")
            .is_none()
    );
    assert_eq!(
        value["microformat"]["availableCountries"],
        json!(["KR", "US"])
    );
}

#[tokio::test]
async fn get_song_rejects_blank_video_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.01.00" });"#,
        ))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let err = client.get_song("   ", 20_000).await.unwrap_err();
    assert!(matches!(
        err,
        Error::InvalidInput(message) if message == "video_id must not be blank"
    ));
}

#[tokio::test]
async fn get_song_posts_player_body_and_returns_typed_response() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.01.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/player"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/song/raw/response1.json")),
        )
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let response = client.get_song("0rilIYWiJ7M", 20_000).await.unwrap();

    assert_eq!(response.video_details.video_id, "0rilIYWiJ7M");
    assert_eq!(response.streaming_data.formats.len(), 1);

    let requests = server.received_requests().await.unwrap();
    let request = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: Value = serde_json::from_slice(&request.body).unwrap();

    assert_eq!(request.url.path(), "/youtubei/v1/player");
    assert_eq!(request.url.query(), Some("alt=json&key=test-api-key"));
    assert_eq!(body["videoId"], "0rilIYWiJ7M");
    assert_eq!(
        body["playbackContext"]["contentPlaybackContext"]["signatureTimestamp"],
        20_000
    );
    assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
    assert_eq!(
        body["context"]["client"]["clientVersion"],
        "1.20250501.01.00"
    );
}
