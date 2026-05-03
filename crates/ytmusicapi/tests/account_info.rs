use std::fs;

use serde_json::json;
use tempfile::tempdir;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};
use ytmusicapi::{AccountInfo, Error, YtMusic, setup_browser_auth};

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

fn account_menu_response() -> serde_json::Value {
    json!({
        "actions": [{
            "openPopupAction": {
                "popup": {
                    "multiPageMenuRenderer": {
                        "header": {
                            "activeAccountHeaderRenderer": {
                                "accountName": {
                                    "runs": [{
                                        "text": "OpenAI Listener"
                                    }]
                                },
                                "channelHandle": {
                                    "runs": [{
                                        "text": "@openai.music"
                                    }]
                                },
                                "accountPhoto": {
                                    "thumbnails": [{
                                        "url": "https://example.com/account.jpg"
                                    }]
                                }
                            }
                        }
                    }
                }
            }
        }]
    })
}

#[tokio::test]
async fn get_account_info_requires_browser_auth() {
    let client = YtMusic::builder().build().unwrap();

    let error = client.get_account_info().await.unwrap_err();
    assert!(matches!(error, Error::UnsupportedFeature(_)));
}

#[tokio::test]
async fn get_account_info_returns_typed_account_info() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"ytcfg.set({ "VISITOR_DATA": "visitor-id-123", "INNERTUBE_API_KEY": "test-api-key", "INNERTUBE_CONTEXT_CLIENT_VERSION": "1.20250501.03.00" });"#,
        ))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/youtubei/v1/account/account_menu"))
        .and(query_param("alt", "json"))
        .and(query_param("key", "test-api-key"))
        .and(body_json(json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(account_menu_response()))
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

    let account_info = client.get_account_info().await.unwrap();

    assert_eq!(
        account_info,
        AccountInfo {
            account_name: "OpenAI Listener".to_owned(),
            channel_handle: Some("@openai.music".to_owned()),
            account_photo_url: "https://example.com/account.jpg".to_owned(),
        }
    );
}
