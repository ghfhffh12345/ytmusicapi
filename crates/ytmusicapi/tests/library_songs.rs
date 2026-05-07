use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    AlbumRef, ArtistRef, Error, LibraryLikeStatus, LibrarySong, LibrarySongsContinuationToken,
    Page, Thumbnail, YtMusic, setup_browser_auth,
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

fn library_songs_continuation_response() -> serde_json::Value {
    json!({
        "continuationContents": {
            "musicShelfContinuation": {
                "contents": [{
                    "musicResponsiveListItemRenderer": {
                        "playlistItemData": { "videoId": "song-3" },
                        "flexColumns": [{
                            "musicResponsiveListItemFlexColumnRenderer": {
                                "text": {
                                    "runs": [{
                                        "text": "Dayvan Cowboy",
                                        "navigationEndpoint": {
                                            "watchEndpoint": { "videoId": "song-3" }
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
                                            "browseEndpoint": { "browseId": "UCBOC" }
                                        }
                                    }, {
                                        "text": " • "
                                    }, {
                                        "text": "The Campfire Headphase",
                                        "navigationEndpoint": {
                                            "browseEndpoint": { "browseId": "MPREb_album_2" }
                                        }
                                    }, {
                                        "text": " • "
                                    }, {
                                        "text": "5:00"
                                    }]
                                }
                            }
                        }],
                        "thumbnail": {
                            "musicThumbnailRenderer": {
                                "thumbnail": {
                                    "thumbnails": [{
                                        "url": "https://example.com/song-3.jpg",
                                        "width": 320,
                                        "height": 320
                                    }]
                                }
                            }
                        }
                    }
                }],
                "continuations": [{
                    "nextContinuationData": {
                        "continuation": "song-token-2"
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
        songs.items,
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
async fn get_library_songs_continuation_returns_page() {
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
            ResponseTemplate::new(200).set_body_json(library_songs_continuation_response()),
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

    let songs = client
        .get_library_songs_continuation(LibrarySongsContinuationToken::new("song-token-1"))
        .await
        .unwrap();
    assert_eq!(
        songs,
        Page {
            items: vec![LibrarySong {
                video_id: "song-3".to_owned(),
                title: "Dayvan Cowboy".to_owned(),
                artists: vec![ArtistRef {
                    id: "UCBOC".to_owned(),
                    name: "Boards of Canada".to_owned(),
                }],
                album: Some(AlbumRef {
                    id: "MPREb_album_2".to_owned(),
                    name: "The Campfire Headphase".to_owned(),
                }),
                duration: Some("5:00".to_owned()),
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/song-3.jpg".to_owned(),
                    width: 320,
                    height: 320,
                }],
                like_status: None,
            }],
            continuation: Some(LibrarySongsContinuationToken::new("song-token-2")),
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
            "continuation": "song-token-1",
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
async fn get_library_songs_skips_localized_leading_control_tile() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][0]["musicResponsiveListItemRenderer"]
        ["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["text"] =
        json!("모두 셔플");
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][0]["musicResponsiveListItemRenderer"]
        ["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"] = json!({
        "browseEndpoint": { "browseId": "RDAMVMshuffle-all" }
    });
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][0]["musicResponsiveListItemRenderer"]
        ["thumbnail"] = json!({
        "musicThumbnailRenderer": {
            "thumbnail": {
                "thumbnails": [{
                    "url": "https://example.com/shuffle-all.jpg",
                    "width": 300,
                    "height": 300
                }]
            }
        }
    });

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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(songs.items.len(), 2);
    assert_eq!(songs.items[0].video_id, "song-1");
    assert_eq!(songs.items[1].video_id, "song-2");
}

#[tokio::test]
async fn get_library_songs_skips_multiple_leading_control_tiles_with_extra_columns() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    let contents = &mut response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]
        ["content"]["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"];

    contents[0]["musicResponsiveListItemRenderer"]["flexColumns"] = json!([
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "모두 셔플" }] }
            }
        },
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "새 작업" }] }
            }
        }
    ]);

    contents.as_array_mut().unwrap().insert(
        1,
        json!({
            "musicResponsiveListItemRenderer": {
                "flexColumns": [{
                    "musicResponsiveListItemFlexColumnRenderer": {
                        "text": { "runs": [{ "text": "Shuffle again" }] }
                    }
                }, {
                    "musicResponsiveListItemFlexColumnRenderer": {
                        "text": { "runs": [{ "text": "Queue next" }] }
                    }
                }]
            }
        }),
    );

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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(songs.items.len(), 2);
    assert_eq!(songs.items[0].video_id, "song-1");
    assert_eq!(songs.items[1].video_id, "song-2");
}

#[tokio::test]
async fn get_library_songs_parses_shifted_combined_metadata_columns() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
        ["flexColumns"] = json!([
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{ "text": "Archangel" }] }
            }
        },
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{
                    "text": "Burial",
                    "navigationEndpoint": {
                        "browseEndpoint": { "browseId": "UCBRL" }
                    }
                }, {
                    "text": " • "
                }, {
                    "text": "Untrue",
                    "navigationEndpoint": {
                        "browseEndpoint": { "browseId": "MPREb_album_2" }
                    }
                }, {
                    "text": " • "
                }, {
                    "text": "6:09"
                }] }
            }
        }
    ]);
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]
        ["musicResponsiveListItemRenderer"]
        .as_object_mut()
        .unwrap()
        .remove("fixedColumns");

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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(
        songs.items[1],
        LibrarySong {
            video_id: "song-2".to_owned(),
            title: "Archangel".to_owned(),
            artists: vec![ArtistRef {
                id: "UCBRL".to_owned(),
                name: "Burial".to_owned(),
            }],
            album: Some(AlbumRef {
                id: "MPREb_album_2".to_owned(),
                name: "Untrue".to_owned(),
            }),
            duration: Some("6:09".to_owned()),
            thumbnails: vec![],
            like_status: None,
        }
    );
}

#[tokio::test]
async fn get_library_songs_parses_title_after_leading_badge_and_multi_run_text() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
        ["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"] = json!([
        { "text": "E" },
        { "text": "Arch" },
        { "text": "angel" }
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(songs.items[1].title, "Archangel");
}

#[tokio::test]
async fn get_library_songs_preserves_plain_text_artist_metadata() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
        ["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"] = json!({
        "watchEndpoint": {
            "videoId": "song-2"
        }
    });
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
        ["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"] = json!([
        { "text": "Burial" }
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(
        songs.items[1].artists,
        vec![ArtistRef {
            id: String::new(),
            name: "Burial".to_owned(),
        }]
    );
}

#[tokio::test]
async fn get_library_songs_preserves_plain_text_album_metadata() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["flexColumns"][0]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"][0]["navigationEndpoint"] = json!({
        "watchEndpoint": {
            "videoId": "song-1"
        }
    });
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["flexColumns"][2]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"] = json!([
        { "text": "Music Has the Right to Children" }
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(
        songs.items[0].album,
        Some(AlbumRef {
            id: String::new(),
            name: "Music Has the Right to Children".to_owned(),
        })
    );
}

#[tokio::test]
async fn get_library_songs_parses_title_when_metadata_column_precedes_it() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
        ["flexColumns"] = json!([
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{
                    "text": "Burial",
                    "navigationEndpoint": {
                        "browseEndpoint": {
                            "browseId": "UCBRL",
                            "browseEndpointContextSupportedConfigs": {
                                "browseEndpointContextMusicConfig": {
                                    "pageType": "MUSIC_PAGE_TYPE_ARTIST"
                                }
                            }
                        }
                    }
                }] }
            }
        },
        {
            "musicResponsiveListItemFlexColumnRenderer": {
                "text": { "runs": [{
                    "text": "Archangel",
                    "navigationEndpoint": {
                        "watchEndpoint": {
                            "videoId": "song-2"
                        }
                    }
                }] }
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(songs.items[1].title, "Archangel");
    assert_eq!(
        songs.items[1].artists,
        vec![ArtistRef {
            id: "UCBRL".to_owned(),
            name: "Burial".to_owned(),
        }]
    );
}

#[tokio::test]
async fn get_library_songs_parses_all_plain_text_columns() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][2]["musicResponsiveListItemRenderer"]
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

    let songs = client.get_library_songs().await.unwrap();
    assert_eq!(
        songs.items[1],
        LibrarySong {
            video_id: "song-2".to_owned(),
            title: "Archangel".to_owned(),
            artists: vec![ArtistRef {
                id: String::new(),
                name: "Burial".to_owned(),
            }],
            album: Some(AlbumRef {
                id: String::new(),
                name: "Untrue".to_owned(),
            }),
            duration: None,
            thumbnails: vec![],
            like_status: None,
        }
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
    assert_eq!(songs.items, Vec::<LibrarySong>::new());
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

#[tokio::test]
async fn get_library_songs_errors_when_first_song_row_is_missing_video_id() {
    let server = MockServer::start().await;
    let mut response = library_songs_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["musicShelfRenderer"]["contents"][0] = json!({
        "musicResponsiveListItemRenderer": {
            "playlistItemData": {},
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
            }]
        }
    });

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
