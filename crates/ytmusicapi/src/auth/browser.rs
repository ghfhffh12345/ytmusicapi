use std::{collections::BTreeMap, fs, path::Path};

use serde::{
    Deserialize,
    de::{self, MapAccess, Visitor},
};

use crate::Error;

pub(crate) type BrowserAuthHeaders = BTreeMap<String, String>;

const DEFAULT_HEADERS: [(&str, &str); 5] = [
    ("user-agent", crate::search::request::USER_AGENT),
    ("accept", "*/*"),
    ("content-type", "application/json"),
    ("content-encoding", "gzip"),
    ("origin", "https://music.youtube.com"),
];

pub fn setup_browser_auth(raw_headers: &str) -> Result<String, Error> {
    let headers = parse_raw_headers(raw_headers)?;
    serde_json::to_string_pretty(&headers).map_err(|source| {
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
    let mut headers = BrowserAuthHeaders::new();

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

fn finalize_headers(mut headers: BrowserAuthHeaders) -> Result<BrowserAuthHeaders, Error> {
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

    Ok(headers)
}

fn is_request_line(line: &str) -> bool {
    const METHODS: [&str; 9] = [
        "GET ", "POST ", "PUT ", "PATCH ", "DELETE ", "HEAD ", "OPTIONS ", "CONNECT ", "TRACE ",
    ];

    METHODS.iter().any(|method| line.starts_with(method)) && line.contains(" HTTP/")
}

fn should_drop_header(name: &str) -> bool {
    matches!(name, "host" | "content-length" | "accept-encoding") || name.starts_with("sec-")
}

fn normalize_header_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn insert_header(
    headers: &mut BrowserAuthHeaders,
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
) -> Result<BrowserAuthHeaders, serde_json::Error> {
    serde_json::from_str::<NormalizedBrowserAuthHeaders>(contents).map(|headers| headers.0)
}

struct NormalizedBrowserAuthHeaders(BrowserAuthHeaders);

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
        let mut headers = BrowserAuthHeaders::new();

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
