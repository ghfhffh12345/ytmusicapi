use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    Error, LibraryPodcast, LibraryPodcastChannel, LibraryPodcastsContinuationToken, Thumbnail,
    YtMusic, setup_browser_auth,
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

fn grid_podcasts_response() -> serde_json::Value {
    json!({
        "contents": {
            "singleColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "selected": true,
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "gridRenderer": {
                                        "items": [{
                                            "musicTwoRowItemRenderer": {
                                                "title": {
                                                    "runs": [{
                                                        "text": "팟캐스트 추가"
                                                    }]
                                                }
                                            }
                                        }, {
                                            "musicTwoRowItemRenderer": {
                                                "title": {
                                                    "runs": [{
                                                        "text": "New Episodes",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "VLRDPN"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "subtitle": {
                                                    "runs": [{
                                                        "text": "YouTube Music"
                                                    }]
                                                },
                                                "thumbnailRenderer": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/new-episodes.jpg",
                                                                "width": 320,
                                                                "height": 320
                                                            }]
                                                        }
                                                    }
                                                }
                                            }
                                        }, {
                                            "musicTwoRowItemRenderer": {
                                                "title": {
                                                    "runs": [{
                                                        "text": "Syntax",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "MPSPpodcast123"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "subtitle": {
                                                    "runs": [{
                                                        "text": "Syntax FM",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "UCpodcastchannel123"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "thumbnailRenderer": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/syntax.jpg",
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
                                                "continuation": "podcast-token-1"
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

fn empty_library_podcasts_response() -> serde_json::Value {
    json!({
        "contents": {
            "singleColumnBrowseResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "selected": true,
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "gridRenderer": {
                                        "items": [{
                                            "musicTwoRowItemRenderer": {
                                                "title": {
                                                    "runs": [{
                                                        "text": "Añadir podcasts"
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

async fn audit_client_with_browse_response(
    body: &'static str,
) -> (MockServer, tempfile::TempDir, YtMusic) {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
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

    (server, dir, client)
}

#[tokio::test]
async fn get_library_podcasts_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_library_podcasts().await.unwrap_err();
    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "get_library_podcasts requires browser authentication"
    ));
}

#[tokio::test]
async fn get_library_podcasts_parses_audit_first_page_subtitle_runs() {
    let (_server, _dir, client) = audit_client_with_browse_response(include_str!(
        "fixtures/audit/raw/library/podcasts/first_page.json"
    ))
    .await;

    let podcasts = client.get_library_podcasts().await.unwrap();

    assert_eq!(podcasts.items.len(), 25);
    assert_eq!(
        podcasts.continuation,
        Some(LibraryPodcastsContinuationToken::new("REDACTED_TOKEN"))
    );
    assert_eq!(podcasts.items[0].channel.id, None);
    assert_eq!(
        podcasts.items[0].channel.name,
        "REDACTED_TEXTREDACTED_TEXTREDACTED_TEXT"
    );
}

#[tokio::test]
async fn get_library_podcasts_audit_continuation_is_terminal() {
    let (_server, _dir, client) = audit_client_with_browse_response(include_str!(
        "fixtures/audit/raw/library/podcasts/continuation.json"
    ))
    .await;

    let podcasts = client
        .get_library_podcasts_continuation(LibraryPodcastsContinuationToken::new("podcast-token-1"))
        .await
        .unwrap();

    assert_eq!(podcasts.items.len(), 14);
    assert_eq!(podcasts.continuation, None);
}

#[tokio::test]
async fn get_library_podcasts_returns_first_page_results() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(grid_podcasts_response()))
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

    let podcasts = client.get_library_podcasts().await.unwrap();
    assert_eq!(
        podcasts.items,
        vec![
            LibraryPodcast {
                title: "New Episodes".to_owned(),
                browse_id: "VLRDPN".to_owned(),
                podcast_id: "RDPN".to_owned(),
                channel: LibraryPodcastChannel {
                    id: None,
                    name: "YouTube Music".to_owned(),
                },
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/new-episodes.jpg".to_owned(),
                    width: 320,
                    height: 320,
                }],
            },
            LibraryPodcast {
                title: "Syntax".to_owned(),
                browse_id: "MPSPpodcast123".to_owned(),
                podcast_id: "podcast123".to_owned(),
                channel: LibraryPodcastChannel {
                    id: Some("UCpodcastchannel123".to_owned()),
                    name: "Syntax FM".to_owned(),
                },
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/syntax.jpg".to_owned(),
                    width: 320,
                    height: 320,
                }],
            }
        ]
    );
    assert_eq!(
        podcasts.continuation,
        Some(LibraryPodcastsContinuationToken::new("podcast-token-1"))
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
            "browseId": "FEmusic_library_non_music_audio_list",
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
async fn get_library_podcasts_returns_empty_results_for_control_tile_only_grid() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_library_podcasts_response()))
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

    let podcasts = client.get_library_podcasts().await.unwrap();
    assert_eq!(podcasts.items, vec![]);
}
