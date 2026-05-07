use serde_json::{Value, json};

use crate::{Error, search::request::BootstrapConfig};

#[allow(dead_code)]
pub(crate) fn build_get_song_body(
    video_id: &str,
    signature_timestamp: u32,
    bootstrap: &BootstrapConfig,
) -> Result<Value, Error> {
    let video_id = video_id.trim();

    if video_id.is_empty() {
        return Err(Error::InvalidInput("video_id must not be blank".to_owned()));
    }

    Ok(json!({
        "videoId": video_id,
        "playbackContext": {
            "contentPlaybackContext": {
                "signatureTimestamp": signature_timestamp
            }
        },
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
                "clientVersion": bootstrap.client_version
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_get_song_body_sets_video_id_signature_timestamp_and_context() {
        let bootstrap = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.03.00".to_owned(),
        };

        let body = build_get_song_body("video-1", 20_000, &bootstrap).unwrap();

        assert_eq!(
            body,
            json!({
                "videoId": "video-1",
                "playbackContext": {
                    "contentPlaybackContext": {
                        "signatureTimestamp": 20_000
                    }
                },
                "context": {
                    "client": {
                        "clientName": "WEB_REMIX",
                        "clientVersion": "1.20250501.03.00"
                    }
                }
            })
        );
    }

    #[test]
    fn build_get_song_body_rejects_blank_video_id() {
        let bootstrap = BootstrapConfig {
            visitor_id: "visitor-id-123".to_owned(),
            innertube_api_key: "test-api-key".to_owned(),
            client_version: "1.20250501.03.00".to_owned(),
        };

        let err = build_get_song_body("   ", 20_000, &bootstrap).unwrap_err();
        assert!(matches!(
            err,
            Error::InvalidInput(message) if message == "video_id must not be blank"
        ));
    }
}
