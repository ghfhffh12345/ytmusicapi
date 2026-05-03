use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{
    ArtistRef, ContinuationToken, Error, LibraryAlbum, Thumbnail, YtMusic, setup_browser_auth,
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

fn library_albums_response() -> serde_json::Value {
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
                                                        "text": "Music Has the Right to Children",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "MPREb_album_1"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "subtitle": {
                                                    "runs": [{
                                                        "text": "Album"
                                                    }, {
                                                        "text": " • "
                                                    }, {
                                                        "text": "Boards of Canada",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "UCBOC"
                                                            }
                                                        }
                                                    }, {
                                                        "text": " • "
                                                    }, {
                                                        "text": "1998"
                                                    }]
                                                },
                                                "menu": {
                                                    "menuRenderer": {
                                                        "items": [{
                                                            "menuNavigationItemRenderer": {
                                                                "navigationEndpoint": {
                                                                    "watchPlaylistEndpoint": {
                                                                        "playlistId": "OLAK5uy_album_1"
                                                                    }
                                                                }
                                                            }
                                                        }]
                                                    }
                                                },
                                                "thumbnailRenderer": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": {
                                                            "thumbnails": [{
                                                                "url": "https://example.com/album-1.jpg",
                                                                "width": 300,
                                                                "height": 300
                                                            }]
                                                        }
                                                    }
                                                }
                                            }
                                        }, {
                                            "musicTwoRowItemRenderer": {
                                                "title": {
                                                    "runs": [{
                                                        "text": "Still Slipping Vol. 1",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "MPREb_album_2"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "subtitle": {
                                                    "runs": [{
                                                        "text": "Single"
                                                    }, {
                                                        "text": " • "
                                                    }, {
                                                        "text": "Joy Orbison",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "UCJOY"
                                                            }
                                                        }
                                                    }, {
                                                        "text": " • "
                                                    }, {
                                                        "text": "Overmono",
                                                        "navigationEndpoint": {
                                                            "browseEndpoint": {
                                                                "browseId": "UCOVR"
                                                            }
                                                        }
                                                    }]
                                                },
                                                "thumbnailRenderer": {
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
                                                "continuation": "album-token-1"
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

fn empty_library_albums_response() -> serde_json::Value {
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
                                                        "text": "No albums yet"
                                                    }]
                                                },
                                                "subtext": {
                                                    "messageSubtextRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "Albums you save to your library will show here"
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
async fn get_library_albums_returns_first_page_results() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(library_albums_response()))
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

    let albums = client.get_library_albums().await.unwrap();
    assert_eq!(
        albums.items,
        vec![
            LibraryAlbum {
                browse_id: "MPREb_album_1".to_owned(),
                playlist_id: Some("OLAK5uy_album_1".to_owned()),
                title: "Music Has the Right to Children".to_owned(),
                type_label: Some("Album".to_owned()),
                artists: vec![ArtistRef {
                    id: "UCBOC".to_owned(),
                    name: "Boards of Canada".to_owned(),
                }],
                year: Some("1998".to_owned()),
                thumbnails: vec![Thumbnail {
                    url: "https://example.com/album-1.jpg".to_owned(),
                    width: 300,
                    height: 300,
                }],
            },
            LibraryAlbum {
                browse_id: "MPREb_album_2".to_owned(),
                playlist_id: None,
                title: "Still Slipping Vol. 1".to_owned(),
                type_label: Some("Single".to_owned()),
                artists: vec![
                    ArtistRef {
                        id: "UCJOY".to_owned(),
                        name: "Joy Orbison".to_owned(),
                    },
                    ArtistRef {
                        id: "UCOVR".to_owned(),
                        name: "Overmono".to_owned(),
                    },
                ],
                year: None,
                thumbnails: vec![],
            }
        ]
    );
    assert_eq!(
        albums.continuation,
        Some(ContinuationToken::new("album-token-1").unwrap())
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
            "browseId": "FEmusic_liked_albums",
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
async fn get_library_albums_preserves_linked_artist_names_that_match_the_year() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    let mut response = library_albums_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["gridRenderer"]["items"][0]["musicTwoRowItemRenderer"]
        ["subtitle"]["runs"] = json!([
        {
            "text": "Album"
        },
        {
            "text": " • "
        },
        {
            "text": "2015",
            "navigationEndpoint": {
                "browseEndpoint": {
                    "browseId": "UC2015"
                }
            }
        },
        {
            "text": " • "
        },
        {
            "text": "2015"
        }
    ]);

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

    let albums = client.get_library_albums().await.unwrap();

    assert_eq!(
        albums.items[0].artists,
        vec![ArtistRef {
            id: "UC2015".to_owned(),
            name: "2015".to_owned(),
        }]
    );
    assert_eq!(albums.items[0].year, Some("2015".to_owned()));
}

#[tokio::test]
async fn get_library_albums_returns_empty_results_for_empty_library_message() {
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
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_library_albums_response()))
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

    let albums = client.get_library_albums().await.unwrap();
    assert!(albums.items.is_empty());
}

#[tokio::test]
async fn get_library_albums_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_library_albums().await.unwrap_err();
    assert!(matches!(error, Error::UnsupportedFeature(_)));
}

#[tokio::test]
async fn get_library_albums_errors_when_grid_item_is_not_an_album_renderer() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    let mut response = library_albums_response();
    response["contents"]["singleColumnBrowseResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
        ["sectionListRenderer"]["contents"][0]["gridRenderer"]["items"][0] = json!({
        "musicResponsiveListItemRenderer": {
            "flexColumns": []
        }
    });

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

    let error = client.get_library_albums().await.unwrap_err();
    assert!(matches!(error, Error::Parse(_)));
}
