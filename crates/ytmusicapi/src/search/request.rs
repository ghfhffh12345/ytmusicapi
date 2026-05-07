use serde_json::{Value, json};

use crate::{Error, SearchQuery};

pub const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36";

const YTCFG_SET_MARKER: &str = "ytcfg.set";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BootstrapConfig {
    pub(crate) visitor_id: String,
    pub(crate) innertube_api_key: String,
    pub(crate) client_version: String,
}

pub async fn bootstrap_config(
    http_client: &reqwest::Client,
    homepage_url: &str,
) -> Result<BootstrapConfig, Error> {
    let response = http_client
        .get(homepage_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(Error::HttpTransport)?;

    let status = response.status();
    let body = response.text().await.map_err(Error::HttpTransport)?;

    if !status.is_success() {
        return Err(Error::HttpStatus {
            status,
            message: body,
        });
    }

    parse_bootstrap_config(&body)
}

pub fn build_search_body(query: &SearchQuery, bootstrap_config: &BootstrapConfig) -> Value {
    let mut body = json!({
        "query": query.query,
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": bootstrap_config.client_version,
            }
        }
    });

    if let Some(params) = query.encoded_params() {
        body["params"] = Value::String(params);
    }

    body
}

pub(crate) fn build_library_body(browse_id: &str, bootstrap_config: &BootstrapConfig) -> Value {
    json!({
        "browseId": browse_id,
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": bootstrap_config.client_version,
            }
        }
    })
}

pub(crate) fn build_continuation_body(
    continuation: &impl crate::model::library::ContinuationValue,
    client_version: &str,
) -> Value {
    json!({
        "continuation": continuation.as_str(),
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": client_version,
            }
        }
    })
}

pub(crate) fn build_library_playlists_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_liked_playlists", bootstrap_config)
}

pub(crate) fn build_library_albums_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_liked_albums", bootstrap_config)
}

pub(crate) fn build_library_artists_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_library_corpus_track_artists", bootstrap_config)
}

pub(crate) fn build_library_subscriptions_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_library_corpus_artists", bootstrap_config)
}

pub(crate) fn build_library_channels_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body(
        "FEmusic_library_non_music_audio_channels_list",
        bootstrap_config,
    )
}

pub(crate) fn build_library_podcasts_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_library_non_music_audio_list", bootstrap_config)
}

pub(crate) fn build_library_songs_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("FEmusic_liked_videos", bootstrap_config)
}

pub(crate) fn build_liked_songs_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("VLLM", bootstrap_config)
}

pub(crate) fn build_saved_episodes_body(bootstrap_config: &BootstrapConfig) -> Value {
    build_library_body("VLSE", bootstrap_config)
}

fn parse_bootstrap_config(body: &str) -> Result<BootstrapConfig, Error> {
    let mut missing_field = None;

    for (start, _) in body.match_indices(YTCFG_SET_MARKER) {
        let Some(remainder) = body.get(start..) else {
            continue;
        };

        if let Some(json) = extract_ytcfg_json(remainder)
            && let Ok(config) = serde_json::from_str::<Value>(json)
        {
            match bootstrap_config_from_value(&config) {
                Ok(config) => return Ok(config),
                Err(Error::MissingVisitorId) => missing_field = Some("VISITOR_DATA"),
                Err(Error::MissingBootstrapField(field)) => missing_field = Some(field),
                Err(_) => {}
            }
        }
    }

    match missing_field {
        Some("VISITOR_DATA") | None => Err(Error::MissingVisitorId),
        Some(field) => Err(Error::MissingBootstrapField(field)),
    }
}

fn bootstrap_config_from_value(config: &Value) -> Result<BootstrapConfig, Error> {
    let visitor_id = config
        .get("VISITOR_DATA")
        .and_then(Value::as_str)
        .ok_or(Error::MissingVisitorId)?;
    let innertube_api_key = config
        .get("INNERTUBE_API_KEY")
        .and_then(Value::as_str)
        .ok_or(Error::MissingBootstrapField("INNERTUBE_API_KEY"))?;
    let client_version = config
        .get("INNERTUBE_CONTEXT_CLIENT_VERSION")
        .and_then(Value::as_str)
        .or_else(|| {
            config
                .pointer("/INNERTUBE_CONTEXT/client/clientVersion")
                .and_then(Value::as_str)
        })
        .ok_or(Error::MissingBootstrapField(
            "INNERTUBE_CONTEXT_CLIENT_VERSION",
        ))?;

    Ok(BootstrapConfig {
        visitor_id: visitor_id.to_owned(),
        innertube_api_key: innertube_api_key.to_owned(),
        client_version: client_version.to_owned(),
    })
}

fn extract_ytcfg_json(remainder: &str) -> Option<&str> {
    let open_paren = remainder.find('(')?;
    let payload = remainder.get(open_paren + 1..)?.trim_start();
    let open_brace = payload.find('{')?;
    let payload = payload.get(open_brace..)?;

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in payload.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }

            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return payload.get(..=index);
                }
            }
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        BootstrapConfig, build_continuation_body, build_library_albums_body,
        build_library_artists_body, build_library_channels_body, build_library_playlists_body,
        build_library_podcasts_body, build_library_songs_body, build_library_subscriptions_body,
        build_liked_songs_body, build_saved_episodes_body,
    };
    use crate::SearchContinuationToken;

    #[test]
    fn build_library_playlists_body_includes_bootstrap_context() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_playlists_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_liked_playlists");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_continuation_body_includes_token_and_context() {
        let body = build_continuation_body(
            &SearchContinuationToken::new("next-page-token"),
            "1.20250501.02.00",
        );

        assert_eq!(
            body,
            serde_json::json!({
                "continuation": "next-page-token",
                "context": {
                    "client": {
                        "clientName": "WEB_REMIX",
                        "clientVersion": "1.20250501.02.00",
                    }
                }
            })
        );
    }

    #[test]
    fn build_library_artists_body_includes_artists_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_artists_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_library_corpus_track_artists");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_library_albums_body_includes_albums_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_albums_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_liked_albums");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_library_subscriptions_body_includes_subscriptions_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_subscriptions_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_library_corpus_artists");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_library_channels_body_includes_channels_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_channels_body(&bootstrap_config);

        assert_eq!(
            body["browseId"],
            "FEmusic_library_non_music_audio_channels_list"
        );
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_library_podcasts_body_includes_podcasts_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_podcasts_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_library_non_music_audio_list");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_library_songs_body_includes_songs_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_library_songs_body(&bootstrap_config);

        assert_eq!(body["browseId"], "FEmusic_liked_videos");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_liked_songs_body_includes_liked_songs_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_liked_songs_body(&bootstrap_config);

        assert_eq!(body["browseId"], "VLLM");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }

    #[test]
    fn build_saved_episodes_body_includes_saved_episodes_browse_id() {
        let bootstrap_config = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.02.00".to_owned(),
        };

        let body = build_saved_episodes_body(&bootstrap_config);

        assert_eq!(body["browseId"], "VLSE");
        assert_eq!(body["context"]["client"]["clientName"], "WEB_REMIX");
        assert_eq!(
            body["context"]["client"]["clientVersion"],
            "1.20250501.02.00"
        );
    }
}
