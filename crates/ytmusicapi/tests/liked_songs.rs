use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    AlbumRef, ArtistRef, Error, LibraryLikeStatus, LikedSongItem, LikedSongsContinuationToken,
    LikedSongsPage, Thumbnail, YtMusic, setup_browser_auth,
};

fn browser_auth_json() -> String {
    setup_browser_auth(
        "POST /youtubei/v1/browse HTTP/3\n\
Host: music.youtube.com\n\
User-Agent: Mozilla/5.0\n\
Accept: */*\n\
Content-Type: application/json\n\
X-Goog-AuthUser: 0\n\
X-Origin: https://music.youtube.com\n\
X-Youtube-Client-Name: 67\n\
X-Youtube-Client-Version: 1.20250501.01.00\n\
Cookie: __Secure-3PAPISID=test-sapisid\n",
    )
    .unwrap()
}

fn liked_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "twoColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicResponsiveHeaderRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Liked Songs"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/liked-songs.jpg",
                                                        "width": 512,
                                                        "height": 512
                                                    }]
                                                }
                                            }
                                        }
                                    }
                                }, {
                                    "musicPlaylistShelfRenderer": {
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "playlistItemData": {
                                                    "videoId": "liked-song-1"
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Roygbiv",
                                                                "navigationEndpoint": {
                                                                    "watchEndpoint": {
                                                                        "videoId": "liked-song-1"
                                                                    }
                                                                }
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Boards of Canada",
                                                                "navigationEndpoint": {
                                                                    "browseEndpoint": {
                                                                        "browseId": "UCBOC"
                                                                    }
                                                                }
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "Music Has the Right to Children",
                                                                "navigationEndpoint": {
                                                                    "browseEndpoint": {
                                                                        "browseId": "MPREb_album_1"
                                                                    }
                                                                }
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "2:31"
                                                            }]
                                                        }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/liked-song-1.jpg",
                                                                "width": 300,
                                                                "height": 300
                                                            }]
                                                        }
                                                    }
                                                },
                                                "menu": {
                                                    "menuRenderer": {
                                                        "topLevelButtons": [{
                                                            "likeButtonRenderer": {
                                                                "likeStatus": "LIKE"
                                                            }
                                                        }]
                                                    }
                                                }
                                            }
                                        }, {
                                            "musicResponsiveListItemRenderer": {
                                                "playlistItemData": {
                                                    "videoId": "liked-song-2"
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Archangel",
                                                                "navigationEndpoint": {
                                                                    "watchEndpoint": {
                                                                        "videoId": "liked-song-2"
                                                                    }
                                                                }
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Burial",
                                                                "navigationEndpoint": {
                                                                    "browseEndpoint": {
                                                                        "browseId": "UCBRL"
                                                                    }
                                                                }
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "6:08"
                                                            }]
                                                        }
                                                    }
                                                }]
                                            }
                                        }],
                                        "continuations": [{
                                            "nextContinuationData": {
                                                "continuation": "liked-token-1"
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
}

fn empty_liked_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "twoColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicResponsiveHeaderRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Liked Songs"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/liked-songs.jpg",
                                                        "width": 512,
                                                        "height": 512
                                                    }]
                                                }
                                            }
                                        }
                                    }
                                }, {
                                    "itemSectionRenderer": {
                                        "contents": [{
                                            "messageRenderer": {
                                                "text": {
                                                    "runs": [{
                                                        "text": "No liked songs yet"
                                                    }]
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
}

fn liked_songs_continuation_response() -> serde_json::Value {
    json!({
        "continuationContents": {
            "musicShelfContinuation": {
                "contents": [{
                    "musicResponsiveListItemRenderer": {
                        "playlistItemData": {
                            "videoId": "liked-song-3"
                        },
                        "flexColumns": [{
                            "musicResponsiveListItemFlexColumnRenderer": {
                                "text": {
                                    "runs": [{
                                        "text": "Windowlicker",
                                        "navigationEndpoint": {
                                            "watchEndpoint": {
                                                "videoId": "liked-song-3"
                                            }
                                        }
                                    }]
                                }
                            }
                        }, {
                            "musicResponsiveListItemFlexColumnRenderer": {
                                "text": {
                                    "runs": [{
                                        "text": "Aphex Twin",
                                        "navigationEndpoint": {
                                            "browseEndpoint": {
                                                "browseId": "UCAPHEX"
                                            }
                                        }
                                    }, {
                                        "text": " • "
                                    }, {
                                        "text": "6:07"
                                    }]
                                }
                            }
                        }]
                    }
                }],
                "continuations": [{
                    "nextContinuationData": {
                        "continuation": "liked-token-2"
                    }
                }]
            }
        }
    })
}

fn empty_liked_songs_simple_text_response() -> serde_json::Value {
    json!({
        "contents": {
            "twoColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicResponsiveHeaderRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Liked Songs"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/liked-songs.jpg",
                                                        "width": 512,
                                                        "height": 512
                                                    }]
                                                }
                                            }
                                        }
                                    }
                                }, {
                                    "itemSectionRenderer": {
                                        "contents": [{
                                            "messageRenderer": {
                                                "text": {
                                                    "simpleText": "Nothing to show here yet"
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
}

fn header_only_empty_liked_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "twoColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicResponsiveHeaderRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Liked Songs"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/liked-songs.jpg",
                                                        "width": 512,
                                                        "height": 512
                                                    }]
                                                }
                                            }
                                        }
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
}

fn generic_error_liked_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "twoColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicResponsiveHeaderRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Liked Songs"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/liked-songs.jpg",
                                                        "width": 512,
                                                        "height": 512
                                                    }]
                                                }
                                            }
                                        }
                                    }
                                }, {
                                    "itemSectionRenderer": {
                                        "contents": [{
                                            "messageRenderer": {
                                                "text": {
                                                    "runs": [{
                                                        "text": "Something went wrong"
                                                    }]
                                                },
                                                "subtext": {
                                                    "messageSubtextRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Try again later"
                                                            }]
                                                        }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
}

#[tokio::test]
async fn get_liked_songs_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_liked_songs().await.unwrap_err();
    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_liked_songs requires browser authentication"
    ));
}

#[tokio::test]
async fn get_liked_songs_returns_typed_wrapper_and_uses_vllm_browse_id() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(liked_songs_response()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert_eq!(
        liked_songs,
        LikedSongsPage {
            playlist_id: "LM".to_owned(),
            title: "Liked Songs".to_owned(),
            items: vec![
                LikedSongItem {
                    video_id: "liked-song-1".to_owned(),
                    title: "Roygbiv".to_owned(),
                    artists: vec![ArtistRef {
                        id: "UCBOC".to_owned(),
                        name: "Boards of Canada".to_owned(),
                    }],
                    album: Some(AlbumRef {
                        id: "MPREb_album_1".to_owned(),
                        name: "Music Has the Right to Children".to_owned(),
                    }),
                    duration: Some("2:31".to_owned()),
                    thumbnails: vec![Thumbnail {
                        url: "https://example.com/liked-song-1.jpg".to_owned(),
                        width: 300,
                        height: 300,
                    }],
                    like_status: Some(LibraryLikeStatus::Like),
                },
                LikedSongItem {
                    video_id: "liked-song-2".to_owned(),
                    title: "Archangel".to_owned(),
                    artists: vec![ArtistRef {
                        id: "UCBRL".to_owned(),
                        name: "Burial".to_owned(),
                    }],
                    album: None,
                    duration: Some("6:08".to_owned()),
                    thumbnails: vec![],
                    like_status: None,
                }
            ],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/liked-songs.jpg".to_owned(),
                width: 512,
                height: 512,
            }],
            continuation: Some(LikedSongsContinuationToken::new("liked-token-1")),
        }
    );

    let requests = server.received_requests().await.unwrap();
    let browse = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: serde_json::Value = browse.body_json().unwrap();
    assert_eq!(
        body,
        json!({
            "browseId": "VLLM",
            "context": {
                "client": {
                    "clientName": "WEB_REMIX",
                    "clientVersion": "1.20250501.01.00"
                }
            }
        })
    );
}

#[tokio::test]
async fn get_liked_songs_continuation_preserves_wrapper_metadata() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(liked_songs_continuation_response()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client
        .get_liked_songs_continuation(LikedSongsContinuationToken::new("liked-token-1"))
        .await
        .unwrap();
    assert_eq!(
        liked_songs,
        LikedSongsPage {
            playlist_id: "LM".to_owned(),
            title: "Liked Songs".to_owned(),
            items: vec![LikedSongItem {
                video_id: "liked-song-3".to_owned(),
                title: "Windowlicker".to_owned(),
                artists: vec![ArtistRef {
                    id: "UCAPHEX".to_owned(),
                    name: "Aphex Twin".to_owned(),
                }],
                album: None,
                duration: Some("6:07".to_owned()),
                thumbnails: vec![],
                like_status: None,
            }],
            thumbnails: vec![],
            continuation: Some(LikedSongsContinuationToken::new("liked-token-2")),
        }
    );
}

#[tokio::test]
async fn get_liked_songs_parses_plain_text_title_metadata_and_long_fixed_duration() {
    let server = MockServer::start().await;
    let mut response = liked_songs_response();
    response["contents"]["twoColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]
        ["contents"][1]["musicPlaylistShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["flexColumns"] = json!([
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "Archangel" }] }
            }
        },
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "Burial" }] }
            }
        },
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "Untrue" }] }
            }
        }
    ]);
    response["contents"]["twoColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]
        ["contents"][1]["musicPlaylistShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["fixedColumns"] = json!([
        {
            "musicResponsiveListItemFixedColumnRenderer": {
                "text": { "runs": [{ "text": "1:02:03" }] }
            }
        }
    ]);

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert_eq!(
        liked_songs.items[1],
        LikedSongItem {
            video_id: "liked-song-2".to_owned(),
            title: "Archangel".to_owned(),
            artists: vec![ArtistRef {
                id: String::new(),
                name: "Burial".to_owned(),
            }],
            album: Some(AlbumRef {
                id: String::new(),
                name: "Untrue".to_owned(),
            }),
            duration: Some("1:02:03".to_owned()),
            thumbnails: vec![],
            like_status: None,
        }
    );
}

#[tokio::test]
async fn get_liked_songs_returns_empty_wrapper_for_empty_library_message() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_liked_songs_response()))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert_eq!(
        liked_songs,
        LikedSongsPage {
            playlist_id: "LM".to_owned(),
            title: "Liked Songs".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/liked-songs.jpg".to_owned(),
                width: 512,
                height: 512,
            }],
            continuation: None,
        }
    );
}

#[tokio::test]
async fn get_liked_songs_returns_empty_wrapper_for_simple_text_message_only_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(empty_liked_songs_simple_text_response()),
        )
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert_eq!(
        liked_songs,
        LikedSongsPage {
            playlist_id: "LM".to_owned(),
            title: "Liked Songs".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/liked-songs.jpg".to_owned(),
                width: 512,
                height: 512,
            }],
            continuation: None,
        }
    );
}

#[tokio::test]
async fn get_liked_songs_returns_empty_wrapper_for_header_only_page() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(header_only_empty_liked_songs_response()),
        )
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert_eq!(
        liked_songs,
        LikedSongsPage {
            playlist_id: "LM".to_owned(),
            title: "Liked Songs".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/liked-songs.jpg".to_owned(),
                width: 512,
                height: 512,
            }],
            continuation: None,
        }
    );
}

#[tokio::test]
async fn get_liked_songs_errors_for_generic_message_only_payload() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "9.99999999.99.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(generic_error_liked_songs_response()),
        )
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let error = client.get_liked_songs().await.unwrap_err();
    assert!(matches!(error, Error::Parse(_)));
}
