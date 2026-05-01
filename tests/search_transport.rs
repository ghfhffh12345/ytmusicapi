use serde_json::{Value, json};
use wiremock::matchers::{body_partial_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{Error, SearchFilter, SearchQuery, YtMusic};

#[tokio::test]
async fn search_bootstraps_visitor_id_and_posts_search_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
                <html>
                  <script>
                    ytcfg.set({
                      "VISITOR_DATA": "visitor-id-123",
                      "INNERTUBE_CONTEXT": {}
                    });
                  </script>
                </html>
                "#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .and(header("x-goog-visitor-id", "visitor-id-123"))
        .and(body_partial_json(json!({
            "query": "hip hop",
            "params": "EgWKAQIYAWoMEA4QChADEAQQCRAF",
            "context": {
                "client": {
                    "clientName": "WEB_REMIX"
                }
            }
        })))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search/raw/albums.json")),
        )
        .mount(&server)
        .await;

    let http_client = reqwest::Client::builder().build().unwrap();
    let client = YtMusic::builder()
        .http_client(http_client)
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let query = SearchQuery::new("hip hop").with_filter(SearchFilter::Albums);
    let result = client.search(query).await.unwrap();
    assert_eq!(
        serde_json::to_value(result).unwrap(),
        serde_json::from_str::<Value>(include_str!("fixtures/search/expected/albums.json"))
            .unwrap()
    );
}

#[tokio::test]
async fn search_reuses_bootstrapped_visitor_id_across_requests() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"window.ytcfg.set({ "VISITOR_DATA" : "visitor-id-123" });"#),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .and(header("x-goog-visitor-id", "visitor-id-123"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(include_str!("fixtures/search/raw/default_mixed.json")),
        )
        .expect(2)
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let first = client.search(SearchQuery::new("first")).await.unwrap();
    let second = client.search(SearchQuery::new("second")).await.unwrap();

    assert_eq!(first.len(), 24);
    assert_eq!(second.len(), 24);
}

#[tokio::test]
async fn missing_visitor_id_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "INNERTUBE_CONTEXT": { "client": {} } });"#),
        )
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client.search(SearchQuery::new("abba")).await.unwrap_err();
    match error {
        Error::MissingVisitorId => {}
        other => panic!("expected MissingVisitorId error, got {other:?}"),
    }
}

#[tokio::test]
async fn invalid_json_response_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123" });"#),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not valid json"))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client.search(SearchQuery::new("abba")).await.unwrap_err();
    match error {
        Error::JsonDecode(_) => {}
        other => panic!("expected JsonDecode error, got {other:?}"),
    }
}

#[tokio::test]
async fn structurally_invalid_json_response_is_parse_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123" });"#),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {}
        })))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client.search(SearchQuery::new("abba")).await.unwrap_err();
    match error {
        Error::Parse(message) => {
            assert!(message.contains("tabbedSearchResultsRenderer"));
        }
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[tokio::test]
async fn empty_successful_search_response_returns_empty_results() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123" });"#),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "tabbedSearchResultsRenderer": {
                    "tabs": []
                }
            }
        })))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let result = client.search(SearchQuery::new("abba")).await.unwrap();

    assert!(result.is_empty());
}

#[tokio::test]
async fn unsupported_filtered_empty_successful_search_is_rejected() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123" });"#),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "contents": {
                "tabbedSearchResultsRenderer": {
                    "tabs": []
                }
            }
        })))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client
        .search(SearchQuery::new("abba").with_filter(SearchFilter::Songs))
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedFeature(message)
            if message == "search parser currently supports only default mixed, albums, artists, and playlists responses"
    ));
}

#[tokio::test]
async fn server_status_is_mapped_to_status_error() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123" });"#),
        )
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
        .and(query_param("alt", "json"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "error": { "message": "boom" }
        })))
        .mount(&server)
        .await;

    let client = YtMusic::builder()
        .homepage_url(server.uri())
        .base_url(format!("{}/youtubei/v1/", server.uri()))
        .build()
        .unwrap();

    let error = client.search(SearchQuery::new("abba")).await.unwrap_err();
    match error {
        Error::HttpStatus { status, .. } => assert_eq!(status.as_u16(), 500),
        other => panic!("expected HttpStatus error, got {other:?}"),
    }
}
