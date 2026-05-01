use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
};
use sha1::{Digest, Sha1};

use crate::Error;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BrowserAuthHeaders {
    pub(crate) headers: BTreeMap<String, String>,
}

impl BrowserAuthHeaders {
    pub(crate) fn to_header_map(
        &self,
        fallback_visitor_id: Option<&str>,
    ) -> Result<HeaderMap, Error> {
        let mut headers = self.headers.clone();

        let cookie = headers.get("cookie").ok_or_else(|| {
            Error::AuthValidation("missing required browser auth header: cookie".to_owned())
        })?;
        let origin = headers
            .get("origin")
            .or_else(|| headers.get("x-origin"))
            .ok_or_else(|| {
                Error::AuthValidation("missing required browser auth header: origin".to_owned())
            })?;
        let sapisid = sapisid_from_cookie(cookie)?;
        headers.insert(
            "authorization".to_owned(),
            build_sapisidhash_authorization(&sapisid, origin),
        );

        if !headers.contains_key("x-goog-visitor-id")
            && let Some(visitor_id) = fallback_visitor_id
        {
            headers.insert("x-goog-visitor-id".to_owned(), visitor_id.to_owned());
        }

        let mut header_map = HeaderMap::new();
        for (name, value) in headers {
            let header_name = HeaderName::from_bytes(name.as_bytes()).map_err(|source| {
                Error::AuthValidation(format!("invalid browser auth header name {name}: {source}"))
            })?;
            let header_value = HeaderValue::from_str(&value).map_err(|source| {
                Error::AuthValidation(format!(
                    "invalid browser auth header value for {name}: {source}"
                ))
            })?;
            header_map.insert(header_name, header_value);
        }

        Ok(header_map)
    }
}

const DEFAULT_HEADERS: [(&str, &str); 5] = [
    ("user-agent", crate::search::request::USER_AGENT),
    ("accept", "*/*"),
    ("content-type", "application/json"),
    ("content-encoding", "gzip"),
    ("origin", "https://music.youtube.com"),
];

pub fn setup_browser_auth(raw_headers: &str) -> Result<String, Error> {
    let headers = parse_raw_headers(raw_headers)?;
    serde_json::to_string_pretty(&headers.headers).map_err(|source| {
        Error::AuthValidation(format!("failed to serialize browser auth json: {source}"))
    })
}

pub(crate) fn load_browser_auth_file(path: &Path) -> Result<BrowserAuthHeaders, Error> {
    let path_display = path.display().to_string();
    let contents = fs::read_to_string(path).map_err(|source| Error::AuthFileRead {
        path: path_display.clone(),
        source,
    })?;
    let headers = deserialize_browser_auth_headers(&contents).map_err(|source| {
        let message = source.to_string();
        if message.starts_with("duplicate browser auth header after normalization: ") {
            return Error::AuthValidation(message);
        }

        Error::AuthFileDecode {
            path: path_display,
            source,
        }
    })?;

    finalize_headers(headers)
}

fn parse_raw_headers(raw_headers: &str) -> Result<BrowserAuthHeaders, Error> {
    let mut headers = BTreeMap::new();

    for line in raw_headers.lines() {
        if line.trim().is_empty() {
            continue;
        }

        if is_request_line(line) {
            continue;
        }

        let Some((name, value)) = line.split_once(": ") else {
            return Err(Error::AuthValidation(
                "malformed browser auth header line".to_owned(),
            ));
        };

        let name = normalize_header_name(name);
        if should_drop_header(&name) {
            continue;
        }

        insert_header(&mut headers, name, value.to_owned())?;
    }

    finalize_headers(headers)
}

fn finalize_headers(mut headers: BTreeMap<String, String>) -> Result<BrowserAuthHeaders, Error> {
    if let Some(x_origin) = headers.get("x-origin").cloned() {
        headers.entry("origin".to_owned()).or_insert(x_origin);
    }

    for (name, value) in DEFAULT_HEADERS {
        headers
            .entry(name.to_owned())
            .or_insert_with(|| value.to_owned());
    }

    for required in ["cookie", "x-goog-authuser"] {
        if !headers.contains_key(required) {
            return Err(Error::AuthValidation(format!(
                "missing required browser auth header: {required}"
            )));
        }
    }

    let cookie = headers.get("cookie").expect("validated cookie header");
    let _ = sapisid_from_cookie(cookie)?;

    Ok(BrowserAuthHeaders { headers })
}

fn is_request_line(line: &str) -> bool {
    const METHODS: [&str; 9] = [
        "GET ", "POST ", "PUT ", "PATCH ", "DELETE ", "HEAD ", "OPTIONS ", "CONNECT ", "TRACE ",
    ];

    METHODS.iter().any(|method| line.starts_with(method)) && line.contains(" HTTP/")
}

fn should_drop_header(name: &str) -> bool {
    matches!(
        name,
        "host" | "content-length" | "accept-encoding" | "authorization"
    ) || name.starts_with("sec-")
}

fn normalize_header_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn insert_header(
    headers: &mut BTreeMap<String, String>,
    name: String,
    value: String,
) -> Result<(), Error> {
    if headers.insert(name.clone(), value).is_some() {
        return Err(Error::AuthValidation(format!(
            "duplicate browser auth header after normalization: {name}"
        )));
    }

    Ok(())
}

fn deserialize_browser_auth_headers(
    contents: &str,
) -> Result<BTreeMap<String, String>, serde_json::Error> {
    serde_json::from_str::<NormalizedBrowserAuthHeaders>(contents).map(|headers| headers.0)
}

struct NormalizedBrowserAuthHeaders(BTreeMap<String, String>);

impl<'de> Deserialize<'de> for NormalizedBrowserAuthHeaders {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(NormalizedBrowserAuthHeadersVisitor)
    }
}

struct NormalizedBrowserAuthHeadersVisitor;

impl<'de> Visitor<'de> for NormalizedBrowserAuthHeadersVisitor {
    type Value = NormalizedBrowserAuthHeaders;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object containing browser auth headers")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut headers = BTreeMap::new();

        while let Some(name) = map.next_key::<String>()? {
            let value = map.next_value::<String>()?;
            let name = normalize_header_name(&name);

            if should_drop_header(&name) {
                continue;
            }

            if headers.insert(name.clone(), value).is_some() {
                return Err(de::Error::custom(format!(
                    "duplicate browser auth header after normalization: {name}"
                )));
            }
        }

        Ok(NormalizedBrowserAuthHeaders(headers))
    }
}

fn sapisid_from_cookie(raw_cookie: &str) -> Result<String, Error> {
    for part in raw_cookie.split(';') {
        let trimmed = part.trim();
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };

        if name == "__Secure-3PAPISID" && !value.is_empty() {
            return Ok(value.to_owned());
        }
    }

    Err(Error::AuthValidation(
        "browser auth cookie must include __Secure-3PAPISID".to_owned(),
    ))
}

fn build_sapisidhash_authorization(sapisid: &str, origin: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs();
    let digest = sha1_hex(format!("{timestamp} {sapisid} {origin}").as_bytes());
    format!("SAPISIDHASH {timestamp}_{digest}")
}

fn sha1_hex(bytes: &[u8]) -> String {
    let digest = Sha1::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::BrowserAuthHeaders;
    use std::collections::BTreeMap;

    #[test]
    fn sha1_hex_matches_known_vector() {
        assert_eq!(
            super::sha1_hex(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn to_header_map_adds_authorization_and_fallback_visitor_id() {
        let headers = BrowserAuthHeaders {
            headers: BTreeMap::from([
                (
                    "cookie".to_owned(),
                    "__Secure-3PAPISID=test-sapisid".to_owned(),
                ),
                ("x-goog-authuser".to_owned(), "0".to_owned()),
                (
                    "x-origin".to_owned(),
                    "https://music.youtube.com".to_owned(),
                ),
                ("origin".to_owned(), "https://music.youtube.com".to_owned()),
                ("x-youtube-client-name".to_owned(), "67".to_owned()),
                (
                    "x-youtube-client-version".to_owned(),
                    "1.20250501.01.00".to_owned(),
                ),
            ]),
        };

        let header_map = headers.to_header_map(Some("visitor-id-123")).unwrap();

        assert_eq!(
            header_map["x-goog-visitor-id"].to_str().unwrap(),
            "visitor-id-123"
        );
        assert_eq!(header_map["x-goog-authuser"].to_str().unwrap(), "0");
        assert!(
            header_map["authorization"]
                .to_str()
                .unwrap()
                .starts_with("SAPISIDHASH ")
        );
    }

    #[test]
    fn to_header_map_overwrites_stale_authorization() {
        let headers = BrowserAuthHeaders {
            headers: BTreeMap::from([
                (
                    "cookie".to_owned(),
                    "__Secure-3PAPISID=test-sapisid".to_owned(),
                ),
                (
                    "authorization".to_owned(),
                    "SAPISIDHASH stale-copy".to_owned(),
                ),
                ("x-goog-authuser".to_owned(), "0".to_owned()),
                (
                    "x-origin".to_owned(),
                    "https://music.youtube.com".to_owned(),
                ),
                ("origin".to_owned(), "https://music.youtube.com".to_owned()),
                ("x-youtube-client-name".to_owned(), "67".to_owned()),
                (
                    "x-youtube-client-version".to_owned(),
                    "1.20250501.01.00".to_owned(),
                ),
            ]),
        };

        let header_map = headers.to_header_map(None).unwrap();
        let authorization = header_map["authorization"].to_str().unwrap();

        assert_ne!(authorization, "SAPISIDHASH stale-copy");
        assert!(authorization.starts_with("SAPISIDHASH "));
    }
}
