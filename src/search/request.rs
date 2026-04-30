use serde_json::{Value, json};

use crate::{Error, SearchQuery};

pub const USER_AGENT: &str = "Mozilla/5.0";

const VISITOR_DATA_MARKER: &str = "\"VISITOR_DATA\":\"";

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
    let (_, remainder) = body.split_once(VISITOR_DATA_MARKER)?;
    let (visitor_id, _) = remainder.split_once('"')?;
    Some(visitor_id.to_owned())
}
