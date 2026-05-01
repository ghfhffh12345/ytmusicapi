use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{ArtistRef, Error, LibraryPlaylist, Thumbnail, YtMusic, setup_browser_auth};

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

fn browser_auth_json_without_client_version() -> String {
    setup_browser_auth(
        "POST /youtubei/v1/browse HTTP/3\n\
Host: music.youtube.com\n\
User-Agent: Mozilla/5.0\n\
Accept: */*\n\
Content-Type: application/json\n\
X-Goog-AuthUser: 0\n\
X-Origin: https://music.youtube.com\n\
X-Youtube-Client-Name: 67\n\
Cookie: __Secure-3PAPISID=test-sapisid\n",
    )
    .unwrap()
}

#[tokio::test]
async fn get_library_playlists_returns_first_page_results() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
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
                                                    "title": { "runs": [{ "text": "Create playlist" }] },
                                                    "subtitle": { "runs": [{ "text": "Control tile" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [] } } }
                                                }
                                            }, {
                                                "musicTwoRowItemRenderer": {
                                                    "title": { "runs": [{ "text": "Synthwave Mix", "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPL123" } } }] },
                                                    "subtitle": { "runs": [{ "text": "OpenAI" }, { "text": " • " }, { "text": "15 songs" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [{ "url": "https://example.com/1.jpg", "width": 300, "height": 300 }] } } }
                                                }
                                            }, {
                                                "musicTwoRowItemRenderer": {
                                                    "title": { "runs": [{ "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPL999" } } }] },
                                                    "subtitle": { "runs": [{ "text": "Archive" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [] } } }
                                                }
                                            }],
                                            "continuations": [{ "nextContinuationData": { "continuation": "ignored-in-this-slice" } }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        })))
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

    let playlists = client.get_library_playlists().await.unwrap();
    assert_eq!(
        playlists,
        vec![
            LibraryPlaylist {
                playlist_id: "PL123".to_owned(),
                title: Some("Synthwave Mix".to_owned()),
                authors: vec![ArtistRef {
                    id: String::new(),
                    name: "OpenAI".to_owned(),
                }],
                item_count: Some(15),
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/1.jpg".to_owned(),
                    width: 300,
                    height: 300,
                }],
            },
            LibraryPlaylist {
                playlist_id: "PL999".to_owned(),
                title: None,
                authors: vec![ArtistRef {
                    id: String::new(),
                    name: "Archive".to_owned(),
                }],
                item_count: None,
                thumbnails: vec![],
            }
        ]
    );
}

#[tokio::test]
async fn get_library_playlists_skips_leading_grid_item() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
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
                                                    "title": { "runs": [{ "text": "Create playlist" }] },
                                                    "subtitle": { "runs": [{ "text": "Control tile" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [] } } }
                                                }
                                            }, {
                                                "musicTwoRowItemRenderer": {
                                                    "title": { "runs": [{ "text": "Actual Playlist", "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPL456" } } }] },
                                                    "subtitle": { "runs": [{ "text": "OpenAI" }, { "text": " • " }, { "text": "8 songs" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [] } } }
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
        })))
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

    let playlists = client.get_library_playlists().await.unwrap();
    assert_eq!(
        playlists,
        vec![LibraryPlaylist {
            playlist_id: "PL456".to_owned(),
            title: Some("Actual Playlist".to_owned()),
            authors: vec![ArtistRef {
                id: String::new(),
                name: "OpenAI".to_owned(),
            }],
            item_count: Some(8),
            thumbnails: vec![],
        }]
    );
}

#[tokio::test]
async fn get_library_playlists_prefers_browser_auth_client_version_in_body() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": []
                                }
                            }
                        }
                    }]
                }
            }
        })))
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

    let playlists = client.get_library_playlists().await.unwrap();
    assert!(playlists.is_empty());

    let requests = server.received_requests().await.unwrap();
    let browse = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&browse.body).unwrap();

    assert_eq!(body["browseId"], "FEmusic_liked_playlists");
    assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
    assert_eq!(
        body["context"]["client"]["clientVersion"],
        "1.20250501.01.00"
    );
}

#[tokio::test]
async fn get_library_playlists_falls_back_to_bootstrap_client_version_in_body() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.09.99" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": []
                                }
                            }
                        }
                    }]
                }
            }
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, browser_auth_json_without_client_version()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let playlists = client.get_library_playlists().await.unwrap();
    assert!(playlists.is_empty());

    let requests = server.received_requests().await.unwrap();
    let browse = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();
    let body: serde_json::Value = serde_json::from_slice(&browse.body).unwrap();

    assert_eq!(
        body["context"]["client"]["clientVersion"],
        "1.20250501.09.99"
    );
}

#[tokio::test]
async fn get_library_playlists_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_library_playlists().await.unwrap_err();
    assert!(matches!(error, Error::UnsupportedFeature(_)));
}

#[tokio::test]
async fn get_library_playlists_ignores_grids_outside_selected_library_tab() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": false,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "gridRenderer": {
                                            "items": [{
                                                "musicTwoRowItemRenderer": {
                                                    "title": { "runs": [{ "text": "Wrong Tab Playlist", "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPLWRONG" } } }] },
                                                    "subtitle": { "runs": [{ "text": "Elsewhere" }, { "text": " • " }, { "text": "3 songs" }] },
                                                    "thumbnailRenderer": { "musicThumbnailRenderer": { "thumbnail": { "thumbnails": [] } } }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }, {
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicShelfRenderer": {
                                            "contents": []
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        })))
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

    let playlists = client.get_library_playlists().await.unwrap();
    assert!(playlists.is_empty());
}

#[tokio::test]
async fn get_library_playlists_errors_when_tabs_are_missing() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {}
            }
        })))
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

    let error = client.get_library_playlists().await.unwrap_err();
    assert!(matches!(error, ytmusicapi::Error::Parse(_)));
}

#[tokio::test]
async fn get_library_playlists_errors_when_single_tab_is_not_selected() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": []
                                }
                            }
                        }
                    }]
                }
            }
        })))
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

    let error = client.get_library_playlists().await.unwrap_err();
    assert!(matches!(error, ytmusicapi::Error::Parse(_)));
}

#[tokio::test]
async fn get_library_playlists_errors_when_library_tab_contents_are_missing() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {}
                            }
                        }
                    }]
                }
            }
        })))
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

    let error = client.get_library_playlists().await.unwrap_err();
    assert!(matches!(error, ytmusicapi::Error::Parse(_)));
}
