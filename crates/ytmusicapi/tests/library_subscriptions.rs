use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    Error, LibrarySubscription, LibrarySubscriptionsContinuationToken, Thumbnail, YtMusic,
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

fn shelf_subscriptions_response() -> serde_json::Value {
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
                                                "navigationEndpoint": {
                                                    "browseEndpoint": {
                                                        "browseId": "UCSubscription1"
                                                    }
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Boards of Canada"
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "1.2M subscribers"
                                                            }]
                                                        }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/subscription-1.jpg",
                                                                "width": 320,
                                                                "height": 320
                                                            }]
                                                        }
                                                    }
                                                }
                                            }
                                        }, {
                                            "musicResponsiveListItemRenderer": {
                                                "navigationEndpoint": {
                                                    "browseEndpoint": {
                                                        "browseId": "UCSubscription2"
                                                    }
                                                },
                                                "flexColumns": [{
                                                    "musicResponsiveListItemFlexColumnRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Burial"
                                                            }]
                                                        }
                                                    }
                                                }],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": []
                                                        }
                                                    }
                                                }
                                            }
                                        }],
                                        "continuations": [{
                                            "nextContinuationData": {
                                                "continuation": "subscription-token-1"
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

fn empty_library_subscriptions_response() -> serde_json::Value {
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
                                                        "text": "No subscriptions yet"
                                                    }]
                                                },
                                                "subtext": {
                                                    "messageSubtextRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Channels you subscribe to will show here"
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
async fn get_library_subscriptions_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_library_subscriptions().await.unwrap_err();
    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_library_subscriptions requires browser authentication"
    ));
}

#[tokio::test]
async fn get_library_subscriptions_returns_first_page_results() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(shelf_subscriptions_response()))
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

    let subscriptions = client.get_library_subscriptions().await.unwrap();
    assert_eq!(
        subscriptions.items,
        vec![
            LibrarySubscription {
                browse_id: "UCSubscription1".to_owned(),
                name: "Boards of Canada".to_owned(),
                subscribers: Some("1.2M".to_owned()),
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/subscription-1.jpg".to_owned(),
                    width: 320,
                    height: 320,
                }],
            },
            LibrarySubscription {
                browse_id: "UCSubscription2".to_owned(),
                name: "Burial".to_owned(),
                subscribers: None,
                thumbnails: vec![],
            }
        ]
    );
    assert_eq!(
        subscriptions.continuation,
        Some(LibrarySubscriptionsContinuationToken::new(
            "subscription-token-1",
        ))
    );

    let requests = server.received_requests().await.unwrap();
    let browse = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&browse.body).unwrap();

    assert_eq!(
        body,
        json!({
            "browseId": "FEmusic_library_corpus_artists",
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
async fn get_library_subscriptions_returns_empty_results_for_empty_library_message() {
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
            ResponseTemplate::new(200).set_body_json(empty_library_subscriptions_response()),
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

    let subscriptions = client.get_library_subscriptions().await.unwrap();
    assert_eq!(subscriptions.items, vec![]);
}
