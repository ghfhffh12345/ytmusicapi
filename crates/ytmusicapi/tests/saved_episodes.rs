use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{Error, SavedEpisodeItem, SavedEpisodes, Thumbnail, YtMusic, setup_browser_auth};

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

fn saved_episodes_response() -> serde_json::Value {
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
                                                "text": "Saved Episodes"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/saved-episodes.jpg",
                                                        "width": 640,
                                                        "height": 640
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
                                                    "videoId": "episode-1"
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Episode 42: The Compiler",
                                                                "navigationEndpoint": {
                                                                    "watchEndpoint": {
                                                                        "videoId": "episode-1"
                                                                    }
                                                                }
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Syntax FM"
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "Syntax"
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "58:10"
                                                            }]
                                                        }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/episode-1.jpg",
                                                                "width": 320,
                                                                "height": 320
                                                            }]
                                                        }
                                                    }
                                                }
                                            }
                                        }, {
                                            "musicResponsiveListItemRenderer": {
                                                "playlistItemData": {
                                                    "videoId": "episode-2"
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Daily Tech Headlines",
                                                                "navigationEndpoint": {
                                                                    "watchEndpoint": {
                                                                        "videoId": "episode-2"
                                                                    }
                                                                }
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Tom Merritt"
                                                            }, {
                                                                "text": " • "
                                                            }, {
                                                                "text": "DTNS"
                                                            }]
                                                        }
                                                    }
                                                }]
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

fn empty_saved_episodes_response() -> serde_json::Value {
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
                                                "text": "Saved Episodes"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/saved-episodes.jpg",
                                                        "width": 640,
                                                        "height": 640
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
                                                        "text": "No saved episodes yet"
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

fn empty_saved_episodes_simple_text_response() -> serde_json::Value {
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
                                                "text": "Saved Episodes"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/saved-episodes.jpg",
                                                        "width": 640,
                                                        "height": 640
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
                                                    "simpleText": "Nothing queued right now"
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

fn header_only_empty_saved_episodes_response() -> serde_json::Value {
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
                                                "text": "Saved Episodes"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/saved-episodes.jpg",
                                                        "width": 640,
                                                        "height": 640
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

fn generic_error_saved_episodes_response() -> serde_json::Value {
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
                                                "text": "Saved Episodes"
                                            }]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": {
                                                    "thumbnails": [{
                                                        "url": "https://example.com/saved-episodes.jpg",
                                                        "width": 640,
                                                        "height": 640
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
async fn get_saved_episodes_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_saved_episodes().await.unwrap_err();
    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_saved_episodes requires browser authentication"
    ));
}

#[tokio::test]
async fn get_saved_episodes_returns_typed_wrapper_and_uses_vlse_browse_id() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(saved_episodes_response()))
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

    let saved_episodes = client.get_saved_episodes().await.unwrap();
    assert_eq!(
        saved_episodes,
        SavedEpisodes {
            playlist_id: "SE".to_owned(),
            title: "Saved Episodes".to_owned(),
            items: vec![
                SavedEpisodeItem {
                    video_id: "episode-1".to_owned(),
                    title: "Episode 42: The Compiler".to_owned(),
                    channel: "Syntax FM".to_owned(),
                    podcast: "Syntax".to_owned(),
                    duration: Some("58:10".to_owned()),
                    thumbnails: vec![Thumbnail {
                        url: "https://example.com/episode-1.jpg".to_owned(),
                        width: 320,
                        height: 320,
                    }],
                },
                SavedEpisodeItem {
                    video_id: "episode-2".to_owned(),
                    title: "Daily Tech Headlines".to_owned(),
                    channel: "Tom Merritt".to_owned(),
                    podcast: "DTNS".to_owned(),
                    duration: None,
                    thumbnails: vec![],
                }
            ],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/saved-episodes.jpg".to_owned(),
                width: 640,
                height: 640,
            }],
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
            "browseId": "VLSE",
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
async fn get_saved_episodes_errors_when_subtitle_metadata_is_incomplete() {
    let server = MockServer::start().await;
    let mut response = saved_episodes_response();
    response["contents"]["twoColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]["sectionListRenderer"]
        ["contents"][1]["musicPlaylistShelfRenderer"]["contents"][1]["musicResponsiveListItemRenderer"]
        ["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"] = json!([
        { "text": "Tom Merritt" }
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

    let error = client.get_saved_episodes().await.unwrap_err();
    assert!(matches!(error, Error::Parse(_)));
}

#[tokio::test]
async fn get_saved_episodes_returns_empty_wrapper_for_empty_library_message() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_saved_episodes_response()))
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

    let saved_episodes = client.get_saved_episodes().await.unwrap();
    assert_eq!(
        saved_episodes,
        SavedEpisodes {
            playlist_id: "SE".to_owned(),
            title: "Saved Episodes".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/saved-episodes.jpg".to_owned(),
                width: 640,
                height: 640,
            }],
        }
    );
}

#[tokio::test]
async fn get_saved_episodes_returns_empty_wrapper_for_simple_text_message_only_page() {
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
            ResponseTemplate::new(200).set_body_json(empty_saved_episodes_simple_text_response()),
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

    let saved_episodes = client.get_saved_episodes().await.unwrap();
    assert_eq!(
        saved_episodes,
        SavedEpisodes {
            playlist_id: "SE".to_owned(),
            title: "Saved Episodes".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/saved-episodes.jpg".to_owned(),
                width: 640,
                height: 640,
            }],
        }
    );
}

#[tokio::test]
async fn get_saved_episodes_returns_empty_wrapper_for_header_only_page() {
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
            ResponseTemplate::new(200).set_body_json(header_only_empty_saved_episodes_response()),
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

    let saved_episodes = client.get_saved_episodes().await.unwrap();
    assert_eq!(
        saved_episodes,
        SavedEpisodes {
            playlist_id: "SE".to_owned(),
            title: "Saved Episodes".to_owned(),
            items: vec![],
            thumbnails: vec![Thumbnail {
                url: "https://example.com/saved-episodes.jpg".to_owned(),
                width: 640,
                height: 640,
            }],
        }
    );
}

#[tokio::test]
async fn get_saved_episodes_errors_for_generic_message_only_payload() {
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
            ResponseTemplate::new(200).set_body_json(generic_error_saved_episodes_response()),
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

    let error = client.get_saved_episodes().await.unwrap_err();
    assert!(matches!(error, Error::Parse(_)));
}
