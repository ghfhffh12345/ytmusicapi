use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::Value;
use ytmusicapi::{SearchFilter, SearchQuery, WatchPlaylistQuery, YtMusic};

// Temporary response-shape audit harness. Keep this narrow and remove it after
// the captured raw corpus has been audited into fixture-backed parser tests.
const CAPTURE_DIR_ENV: &str = "YTMUSICAPI_CAPTURE_DIR";
const CAPTURE_LABEL_ENV: &str = "YTMUSICAPI_CAPTURE_LABEL";
const SIGNATURE_TIMESTAMP: u32 = 20_577;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = manifest_dir.join("tests/fixtures");
    let scratch_dir = workspace_root().join("target/capture_raw_fixtures");
    fs::create_dir_all(&scratch_dir)?;

    let anonymous = YtMusic::new()?;
    capture_anonymous_search(&anonymous, &scratch_dir, &fixture_root).await?;
    capture_watch(&anonymous, &scratch_dir, &fixture_root).await?;
    capture_songs(&anonymous, &scratch_dir, &fixture_root).await?;

    let browser_json = browser_json_path();
    if browser_json.exists() {
        let authenticated = YtMusic::from_browser_auth_file(&browser_json)?;
        capture_authenticated_search(&authenticated, &scratch_dir, &fixture_root).await?;
        capture_library_and_account(&authenticated, &scratch_dir, &fixture_root).await?;
    } else {
        eprintln!(
            "skipping authenticated captures because {} does not exist",
            browser_json.display()
        );
    }

    Ok(())
}

async fn capture_anonymous_search(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn Error>> {
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/default_mixed.json",
        Some("audit/raw/search/default_mixed_continuation.json"),
        SearchQuery::new("abba"),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/albums.json",
        Some("audit/raw/search/albums_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Albums),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/artists.json",
        Some("audit/raw/search/artists_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Artists),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/playlists.json",
        Some("audit/raw/search/playlists_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Playlists),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/songs.json",
        Some("audit/raw/search/songs_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Songs),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/videos.json",
        Some("audit/raw/search/videos_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Videos),
    )
    .await
}

async fn capture_authenticated_search(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn Error>> {
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/songs_authenticated.json",
        Some("audit/raw/search/songs_authenticated_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Songs),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "audit/raw/search/videos_authenticated.json",
        Some("audit/raw/search/videos_authenticated_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Videos),
    )
    .await
}

async fn capture_search(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
    first_page_fixture: &str,
    continuation_fixture: Option<&str>,
    query: SearchQuery,
) -> Result<(), Box<dyn Error>> {
    let Some(page) = capture(
        scratch_dir,
        fixture_root,
        first_page_fixture,
        "search",
        client.search(query),
    )
    .await?
    else {
        return Ok(());
    };

    let Some(continuation_fixture) = continuation_fixture else {
        return Ok(());
    };
    if let Some(token) = page.continuation {
        capture(
            scratch_dir,
            fixture_root,
            continuation_fixture,
            "search",
            client.search_continuation(token),
        )
        .await?;
    } else {
        eprintln!("{first_page_fixture} returned no continuation token");
    }

    Ok(())
}

async fn capture_watch(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn Error>> {
    let first_page = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/watch/first_page.json",
        "next",
        client.get_watch_playlist(WatchPlaylistQuery::new().with_video_id("4y33h81phKU")),
    )
    .await?;
    if let Some(page) = first_page {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/watch/continuation.json",
                "next",
                client.get_watch_playlist_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/watch/first_page.json returned no continuation token");
        }
    }

    capture(
        scratch_dir,
        fixture_root,
        "audit/raw/watch/radio_first_page.json",
        "next",
        client.get_watch_playlist(
            WatchPlaylistQuery::new()
                .with_video_id("4y33h81phKU")
                .radio(),
        ),
    )
    .await?;
    capture(
        scratch_dir,
        fixture_root,
        "audit/raw/watch/shuffle_first_page.json",
        "next",
        client.get_watch_playlist(
            WatchPlaylistQuery::new()
                .with_playlist_id("PL4fGSI1pDJn5kI81J1fYWK5eZRl1zJ5kM")
                .shuffle(),
        ),
    )
    .await?;

    Ok(())
}

async fn capture_songs(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn Error>> {
    for (fixture, video_id) in [
        ("audit/raw/song/response1.json", "4y33h81phKU"),
        ("audit/raw/song/response2.json", "LhiRts68_bk"),
        ("audit/raw/song/response3.json", "Zi_XLOBDo_Y"),
    ] {
        capture(
            scratch_dir,
            fixture_root,
            fixture,
            "player",
            client.get_song(video_id, SIGNATURE_TIMESTAMP),
        )
        .await?;
    }

    Ok(())
}

async fn capture_library_and_account(
    client: &YtMusic,
    scratch_dir: &Path,
    fixture_root: &Path,
) -> Result<(), Box<dyn Error>> {
    capture(
        scratch_dir,
        fixture_root,
        "audit/raw/account/account_info.json",
        "account_account_menu",
        client.get_account_info(),
    )
    .await?;

    let playlists = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/playlists/first_page.json",
        "browse",
        client.get_library_playlists(),
    )
    .await?;
    if let Some(page) = playlists {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/playlists/continuation.json",
                "browse",
                client.get_library_playlists_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/playlists/first_page.json returned no continuation token");
        }
    }

    let artists = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/artists/first_page.json",
        "browse",
        client.get_library_artists(),
    )
    .await?;
    if let Some(page) = artists {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/artists/continuation.json",
                "browse",
                client.get_library_artists_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/artists/first_page.json returned no continuation token");
        }
    }

    let albums = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/albums/first_page.json",
        "browse",
        client.get_library_albums(),
    )
    .await?;
    if let Some(page) = albums {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/albums/continuation.json",
                "browse",
                client.get_library_albums_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/albums/first_page.json returned no continuation token");
        }
    }

    let subscriptions = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/subscriptions/first_page.json",
        "browse",
        client.get_library_subscriptions(),
    )
    .await?;
    if let Some(page) = subscriptions {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/subscriptions/continuation.json",
                "browse",
                client.get_library_subscriptions_continuation(token),
            )
            .await?;
        } else {
            eprintln!(
                "audit/raw/library/subscriptions/first_page.json returned no continuation token"
            );
        }
    }

    let channels = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/channels/first_page.json",
        "browse",
        client.get_library_channels(),
    )
    .await?;
    if let Some(page) = channels {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/channels/continuation.json",
                "browse",
                client.get_library_channels_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/channels/first_page.json returned no continuation token");
        }
    }

    let podcasts = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/podcasts/first_page.json",
        "browse",
        client.get_library_podcasts(),
    )
    .await?;
    if let Some(page) = podcasts {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/podcasts/continuation.json",
                "browse",
                client.get_library_podcasts_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/podcasts/first_page.json returned no continuation token");
        }
    }

    let songs = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/songs/first_page.json",
        "browse",
        client.get_library_songs(),
    )
    .await?;
    if let Some(page) = songs {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/songs/continuation.json",
                "browse",
                client.get_library_songs_continuation(token),
            )
            .await?;
        } else {
            eprintln!("audit/raw/library/songs/first_page.json returned no continuation token");
        }
    }

    let liked_songs = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/liked_songs/first_page.json",
        "browse",
        client.get_liked_songs(),
    )
    .await?;
    if let Some(page) = liked_songs {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/liked_songs/continuation.json",
                "browse",
                client.get_liked_songs_continuation(token),
            )
            .await?;
        } else {
            eprintln!(
                "audit/raw/library/liked_songs/first_page.json returned no continuation token"
            );
        }
    }

    let saved_episodes = capture(
        scratch_dir,
        fixture_root,
        "audit/raw/library/saved_episodes/first_page.json",
        "browse",
        client.get_saved_episodes(),
    )
    .await?;
    if let Some(page) = saved_episodes {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "audit/raw/library/saved_episodes/continuation.json",
                "browse",
                client.get_saved_episodes_continuation(token),
            )
            .await?;
        } else {
            eprintln!(
                "audit/raw/library/saved_episodes/first_page.json returned no continuation token"
            );
        }
    }

    Ok(())
}

async fn capture<T, Fut>(
    scratch_dir: &Path,
    fixture_root: &Path,
    fixture: &str,
    endpoint_file_stem: &str,
    future: Fut,
) -> Result<Option<T>, Box<dyn Error>>
where
    Fut: Future<Output = Result<T, ytmusicapi::Error>>,
{
    let label = capture_label(fixture);
    let label_dir = scratch_dir.join(&label);
    if label_dir.exists() {
        fs::remove_dir_all(&label_dir)?;
    }

    set_capture_env(scratch_dir, &label);
    let result = future.await;
    clear_capture_env();

    let captured = label_dir.join(format!("{endpoint_file_stem}.json"));
    let target = fixture_root.join(fixture);
    let copied = if captured.is_file() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        write_captured_fixture(&captured, &target, fixture)?;
        println!("captured {fixture}");
        true
    } else {
        false
    };

    match result {
        Ok(value) if copied => Ok(Some(value)),
        Ok(_) => Err(io_error(format!(
            "method completed but did not write expected capture file {}",
            captured.display()
        ))),
        Err(error) if copied => {
            eprintln!("captured {fixture}, but method returned {error}");
            Ok(None)
        }
        Err(error) => Err(Box::new(error)),
    }
}

fn set_capture_env(dir: &Path, label: &str) {
    unsafe {
        env::set_var(CAPTURE_DIR_ENV, dir);
        env::set_var(CAPTURE_LABEL_ENV, label);
    }
}

fn clear_capture_env() {
    unsafe {
        env::remove_var(CAPTURE_DIR_ENV);
        env::remove_var(CAPTURE_LABEL_ENV);
    }
}

fn capture_label(fixture: &str) -> String {
    fixture
        .trim_end_matches(".json")
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '_',
        })
        .collect()
}

fn write_captured_fixture(
    captured: &Path,
    target: &Path,
    fixture: &str,
) -> Result<(), Box<dyn Error>> {
    let mut value: Value = serde_json::from_reader(File::open(captured)?)?;
    redact_for_fixture(fixture, &mut value);

    let mut file = File::create(target)?;
    serde_json::to_writer_pretty(&mut file, &value)?;
    file.write_all(b"\n")?;
    Ok(())
}

fn redact_for_fixture(fixture: &str, value: &mut Value) {
    match redaction_mode(fixture) {
        RedactionMode::None => {}
        mode => redact_value(value, mode, None, false),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RedactionMode {
    None,
    SensitiveFields,
    AllStrings,
}

fn redaction_mode(fixture: &str) -> RedactionMode {
    if fixture.starts_with("audit/raw/account/") || fixture.starts_with("audit/raw/library/") {
        RedactionMode::AllStrings
    } else if fixture.contains("_authenticated") {
        RedactionMode::SensitiveFields
    } else {
        RedactionMode::None
    }
}

fn redact_value(
    value: &mut Value,
    mode: RedactionMode,
    parent_key: Option<&str>,
    sensitive_ancestor: bool,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let is_sensitive = sensitive_ancestor
                    || matches!(mode, RedactionMode::AllStrings)
                    || matches!(mode, RedactionMode::SensitiveFields) && is_sensitive_key(key);
                redact_value(child, mode, Some(key), is_sensitive);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, mode, parent_key, sensitive_ancestor);
            }
        }
        Value::String(text) if should_redact_string(mode, parent_key, sensitive_ancestor) => {
            *text = redacted_string(parent_key.unwrap_or("value"));
        }
        _ => {}
    }
}

fn should_redact_string(mode: RedactionMode, key: Option<&str>, sensitive_ancestor: bool) -> bool {
    match mode {
        RedactionMode::None => false,
        RedactionMode::AllStrings => true,
        RedactionMode::SensitiveFields => sensitive_ancestor || key.is_some_and(is_sensitive_key),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("url")
        || key.contains("uri")
        || key.contains("continuation")
        || key.contains("token")
        || key.contains("tracking")
        || key.contains("params")
        || key.contains("visitor")
        || key.contains("account")
        || key.contains("credential")
        || key.contains("auth")
        || key.contains("cookie")
        || key.contains("serializedshareentity")
}

fn redacted_string(key: &str) -> String {
    let key = key.to_ascii_lowercase();
    if key.contains("url") || key.contains("uri") {
        "https://example.invalid/redacted".to_owned()
    } else if key.contains("continuation") || key.contains("token") || key.contains("params") {
        "REDACTED_TOKEN".to_owned()
    } else if key.contains("id") {
        "REDACTED_ID".to_owned()
    } else {
        "REDACTED_TEXT".to_owned()
    }
}

fn browser_json_path() -> PathBuf {
    repo_root().join("browser.json")
}

fn repo_root() -> PathBuf {
    let worktree_root = workspace_root();
    worktree_root
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == ".worktrees"))
        .and_then(|worktrees_dir| worktrees_dir.parent())
        .map(Path::to_path_buf)
        .unwrap_or(worktree_root)
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn io_error(message: String) -> Box<dyn Error> {
    Box::new(std::io::Error::other(message))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::redact_for_fixture;

    #[test]
    fn library_audit_fixtures_redact_personal_display_content_and_urls() {
        let mut value = json!({
            "title": "My private playlist",
            "videoId": "saved-video-id",
            "thumbnail": {
                "url": "https://yt3.googleusercontent.com/private"
            },
            "runs": [
                { "text": "Private Artist" }
            ]
        });

        redact_for_fixture("audit/raw/library/songs/first_page.json", &mut value);

        assert_eq!(value["title"], "REDACTED_TEXT");
        assert_eq!(value["videoId"], "REDACTED_ID");
        assert_eq!(
            value["thumbnail"]["url"],
            "https://example.invalid/redacted"
        );
        assert_eq!(value["runs"][0]["text"], "REDACTED_TEXT");
    }

    #[test]
    fn authenticated_search_redacts_tokens_but_keeps_public_result_text() {
        let mut value = json!({
            "title": "Dancing Queen",
            "continuation": "session-token",
            "clickTrackingParams": "tracking-token",
            "thumbnail": {
                "url": "https://yt3.googleusercontent.com/public"
            }
        });

        redact_for_fixture("audit/raw/search/songs_authenticated.json", &mut value);

        assert_eq!(value["title"], "Dancing Queen");
        assert_eq!(value["continuation"], "REDACTED_TOKEN");
        assert_eq!(value["clickTrackingParams"], "REDACTED_TOKEN");
        assert_eq!(
            value["thumbnail"]["url"],
            "https://example.invalid/redacted"
        );
    }

    #[test]
    fn authenticated_search_redacts_values_under_sensitive_ancestors() {
        let mut value = json!({
            "serviceTrackingParams": [
                {
                    "service": "CSI",
                    "params": [
                        {
                            "key": "c",
                            "value": "WEB_REMIX"
                        },
                        {
                            "key": "yt_li",
                            "value": "1"
                        }
                    ]
                }
            ],
            "title": "Dancing Queen"
        });

        redact_for_fixture("audit/raw/search/songs_authenticated.json", &mut value);

        assert_eq!(
            value["serviceTrackingParams"][0]["service"],
            "REDACTED_TEXT"
        );
        assert_eq!(
            value["serviceTrackingParams"][0]["params"][0]["key"],
            "REDACTED_TEXT"
        );
        assert_eq!(
            value["serviceTrackingParams"][0]["params"][0]["value"],
            "REDACTED_TEXT"
        );
        assert_eq!(value["title"], "Dancing Queen");
    }
}
