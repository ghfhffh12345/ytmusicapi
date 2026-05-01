use std::{collections::BTreeMap, fs, path::Path};

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
    let headers = serde_json::from_str::<BrowserAuthHeaders>(&contents).map_err(|source| {
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
            return Err(Error::AuthValidation(format!(
                "malformed browser auth header line: {line}"
            )));
        };

        let name = name.to_ascii_lowercase();
        if should_drop_header(&name) {
            continue;
        }

        headers.insert(name, value.to_owned());
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
