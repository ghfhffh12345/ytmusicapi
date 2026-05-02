use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    AlbumRef, ArtistRef, Error, LibraryLikeStatus, LibrarySong, Thumbnail, YtMusic,
    setup_browser_auth,
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

fn library_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "singleColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "selected": true,
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicShelfRenderer": {
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{ "text": "Shuffle all" }] }
                                                    }
                                                }]
                                            }
                                        }, {
                                            "musicResponsiveListItemRenderer": {
                                                "playlistItemData": { "videoId": "song-1" },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{ "text": "Roygbiv" }] }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{
                                                            "text": "Boards of Canada",
                                                            "navigationEndpoint": {
                                                                "browseEndpoint": { "browseId": "UCBOC" }
                                                            }
                                                        }] }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{
                                                            "text": "Music Has the Right to Children",
                                                            "navigationEndpoint": {
                                                                "browseEndpoint": { "browseId": "MPREb_album_1" }
                                                            }
                                                        }] }
                                                    }
                                                }],
                                                "fixedColumns": [{
                                                    "musicResponsiveListItemFixedColumnRenderer": {
                                                        "text": { "runs": [{ "text": "2:31" }] }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/song-1.jpg",
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
                                                "playlistItemData": { "videoId": "song-2" },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{ "text": "Archangel" }] }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": { "runs": [{
                                                            "text": "Burial",
                                                            "navigationEndpoint": {
                                                                "browseEndpoint": { "browseId": "UCBRL" }
                                                            }
                                                        }] }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
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

fn empty_library_songs_response() -> serde_json::Value {
    json!({
        "contents": {
            "singleColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "selected": true,
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "itemSectionRenderer": {
                                        "contents": [{
                                            "messageRenderer": {
                                                "text": {
                                                    "runs": [{
                                                        "text": "No songs yet"
                                                    }]
                                                },
                                                "subtext": {
                                                    "messageSubtextRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Songs you save to your library will show here"
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
async fn get_library_songs_requires_browser_auth() {
    let client = YtMusic::new().unwrap();
    let error = client.get_library_songs().await.unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_library_songs requires browser authentication"
    ));
}

#[tokio::test]
async fn get_library_songs_returns_first_page_results() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(library_songs_response()))
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(
        songs,
        vec![
            LibrarySong {
                video_id: "song-1".to_owned(),
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
                    url: "https://example.com/song-1.jpg".to_owned(),
                    width: 300,
                    height: 300,
                }],
                like_status: Some(LibraryLikeStatus::Like),
            },
            LibrarySong {
                video_id: "song-2".to_owned(),
                title: "Archangel".to_owned(),
                artists: vec![ArtistRef {
                    id: "UCBRL".to_owned(),
                    name: "Burial".to_owned(),
                }],
                album: None,
                duration: None,
                thumbnails: vec![],
                like_status: None,
            }
        ]
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
            "browseId": "FEmusic_liked_videos",
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
async fn get_library_songs_returns_empty_results_for_empty_library_message() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_library_songs_response()))
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(songs, Vec::<LibrarySong>::new());
}

#[tokio::test]
async fn get_library_songs_errors_when_non_leading_song_row_is_missing_video_id() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["playlistItemData"] = json!({});

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

    let error = client.get_library_songs().await.unwrap_err();
    assert!(matches!(error, Error::Parse(_)));
}
