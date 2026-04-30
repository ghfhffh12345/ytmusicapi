use serde_json::{Value, json};

use crate::{Error, SearchQuery};

pub const USER_AGENT: &str = "Mozilla/5.0";

const YTCFG_SET_MARKER: &str = "ytcfg.set";

pub async fn bootstrap_visitor_id(
    http_client: &reqwest::Client,
    homepage_url: &str,
) -> Result<String, Error> {
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

    parse_visitor_id(&body).ok_or(Error::MissingVisitorId)
}

pub fn build_search_body(query: &SearchQuery) -> Value {
    let mut body = json!({
        "query": query.query,
        "context": {
            "client": {
                "clientName": "WEB_REMIX",
            }
        }
    });

    if let Some(params) = query.encoded_params() {
        body["params"] = Value::String(params);
    }

    body
}

fn parse_visitor_id(body: &str) -> Option<String> {
    for (start, _) in body.match_indices(YTCFG_SET_MARKER) {
        let remainder = body.get(start..)?;
        if let Some(json) = extract_ytcfg_json(remainder) {
            if let Ok(config) = serde_json::from_str::<Value>(json) {
                if let Some(visitor_id) = config.get("VISITOR_DATA").and_then(Value::as_str) {
                    return Some(visitor_id.to_owned());
                }
            }
        }
    }

    None
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
