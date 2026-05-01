use serde_json::{Value, json};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{Error, SearchFilter, SearchQuery, YtMusic};

const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

#[tokio::test]
async fn search_bootstraps_visitor_id_and_posts_search_request() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"
                <html>
                  <script>
                    ytcfg.set({ "INNERTUBE_CONTEXT": {} });
                    ytcfg.set({
                      "VISITOR_DATA": "visitor-id-123",
                      "INNERTUBE_API_KEY": "test-api-key",
                      "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.01.00"
                    });
                  </script>
                </html>
                "#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
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

    let requests = server.received_requests().await.unwrap();
    let bootstrap_request = requests
        .iter()
        .find(|request| request.method.as_str() == "GET")
        .expect("expected bootstrap GET request");
    let search_request = requests
        .iter()
        .find(|request| request.method.as_str() == "POST")
        .expect("expected search POST request");
    let search_body: Value = serde_json::from_slice(&search_request.body).unwrap();

    assert_eq!(bootstrap_request.url.path(), "/");
    assert_eq!(
        bootstrap_request
            .headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok()),
        Some(BROWSER_USER_AGENT)
    );
    assert_eq!(search_request.url.path(), "/youtubei/v1/search");
    assert_eq!(
        search_request.url.query(),
        Some("alt=json&key=test-api-key")
    );
    assert_eq!(
        search_request
            .headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok()),
        Some(BROWSER_USER_AGENT)
    );
    assert_eq!(
        search_request
            .headers
            .get("x-goog-visitor-id")
            .and_then(|value| value.to_str().ok()),
        Some("visitor-id-123")
    );
    assert_eq!(search_body["query"], "hip hop");
    assert_eq!(search_body["params"], "EgWKAQIYAWoMEA4QChADEAQQCRAF");
    assert_eq!(search_body["context"]["client"]["clientName"], "WEB_REMIX");
    assert_eq!(
        search_body["context"]["client"]["clientVersion"],
        "1.20250501.01.00"
    );
}

#[tokio::test]
async fn search_reuses_bootstrapped_visitor_id_across_requests() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"window.ytcfg.set({ "VISITOR_DATA" : "visitor-id-123", "INNERTUBE_API_KEY": "cached-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.02.00" });"#,
            ),
        )
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/search"))
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

    let requests = server.received_requests().await.unwrap();
    let bootstrap_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.method.as_str() == "GET")
        .collect();
    let post_requests: Vec<_> = requests
        .iter()
        .filter(|request| request.method.as_str() == "POST")
        .collect();
    assert_eq!(bootstrap_requests.len(), 1);
    assert_eq!(post_requests.len(), 2);

    assert_eq!(bootstrap_requests[0].url.path(), "/");
    assert_eq!(
        bootstrap_requests[0]
            .headers
            .get("user-agent")
            .and_then(|value| value.to_str().ok()),
        Some(BROWSER_USER_AGENT)
    );

    for request in post_requests {
        let body: Value = serde_json::from_slice(&request.body).unwrap();

        assert_eq!(request.url.path(), "/youtubei/v1/search");
        assert_eq!(request.url.query(), Some("alt=json&key=cached-api-key"));
        assert_eq!(
            request
                .headers
                .get("x-goog-visitor-id")
                .and_then(|value| value.to_str().ok()),
            Some("visitor-id-123")
        );
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }
}

#[tokio::test]
async fn missing_bootstrap_field_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
            ),
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
        Error::MissingBootstrapField(field) => assert_eq!(field, "INNERTUBE_API_KEY"),
        other => panic!("expected MissingBootstrapField error, got {other:?}"),
    }
}

#[tokio::test]
async fn invalid_json_response_is_reported() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.04.00" });"#,
            ),
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
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.05.00" });"#,
            ),
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
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.06.00" });"#,
            ),
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
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.07.00" });"#,
            ),
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
            ResponseTemplate::new(200).set_body_string(
                r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.08.00" });"#,
            ),
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
