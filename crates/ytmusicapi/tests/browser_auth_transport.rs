use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{Error, YtMusic, setup_browser_auth};

fn auth_json() -> String {
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

#[tokio::test]
async fn authenticated_client_posts_browse_with_browser_headers() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.02.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/browse"))
        .and(query_param("alt", "json"))
        .and(query_param("key", "test-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": { "singleColumnBrowseResultsRenderer": { "tabs": [] } }
        })))
        .mount(&server)
        .await;

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, auth_json()).unwrap();

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .browser_auth_path(&path)
        .build()
        .unwrap();

    let _ = client.get_library_playlists().await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let browse = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .unwrap();

    assert_eq!(browse.url.path(), "/youtubei/v1/browse");
    assert_eq!(browse.headers["x-goog-authuser"], "0");
    assert_eq!(browse.headers["x-origin"], "https://music.youtube.com");
    assert_eq!(browse.headers["x-goog-visitor-id"], "visitor-id-123");
    assert_eq!(browse.headers["x-youtube-client-name"], "67");
    assert_eq!(
        browse.headers["x-youtube-client-version"],
        "1.20250501.01.00"
    );
    assert!(
        browse.headers["authorization"]
            .to_str()
            .unwrap()
            .starts_with("SAPISIDHASH ")
    );
}

#[test]
fn browser_auth_cookie_requires_secure_3papisid() {
    let json = serde_json::json!({
        "cookie": "VISITOR_PRIVACY_METADATA=CgJVUxIEGgAgVg%3D%3D",
        "x-goog-authuser": "0",
        "x-origin": "https://music.youtube.com",
        "x-youtube-client-name": "67",
        "x-youtube-client-version": "1.20250501.01.00"
    })
    .to_string();

    let dir = tempdir().unwrap();
    let path = dir.path().join("browser.json");
    fs::write(&path, json).unwrap();

    let error = YtMusic::from_browser_auth_file(&path).unwrap_err();
    assert!(matches!(error, Error::AuthValidation(_)));
}
