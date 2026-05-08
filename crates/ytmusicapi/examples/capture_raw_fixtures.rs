use std::env;
use std::error::Error;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

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
        "search/raw/default_mixed.json",
        Some("search/raw/default_mixed_continuation.json"),
        SearchQuery::new("abba"),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/albums.json",
        Some("search/raw/albums_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Albums),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/artists.json",
        Some("search/raw/artists_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Artists),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/playlists.json",
        Some("search/raw/playlists_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Playlists),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/songs.json",
        Some("search/raw/songs_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Songs),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/videos.json",
        Some("search/raw/videos_continuation.json"),
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
        "search/raw/songs_authenticated.json",
        Some("search/raw/songs_authenticated_continuation.json"),
        SearchQuery::new("abba").with_filter(SearchFilter::Songs),
    )
    .await?;
    capture_search(
        client,
        scratch_dir,
        fixture_root,
        "search/raw/videos_authenticated.json",
        Some("search/raw/videos_authenticated_continuation.json"),
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
        "watch/raw/first_page.json",
        "next",
        client.get_watch_playlist(WatchPlaylistQuery::new().with_video_id("4y33h81phKU")),
    )
    .await?;
    if let Some(page) = first_page {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "watch/raw/continuation.json",
                "next",
                client.get_watch_playlist_continuation(token),
            )
            .await?;
        } else {
            eprintln!("watch/raw/first_page.json returned no continuation token");
        }
    }

    capture(
        scratch_dir,
        fixture_root,
        "watch/raw/radio_first_page.json",
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
        "watch/raw/shuffle_first_page.json",
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
        ("song/raw/response1.json", "4y33h81phKU"),
        ("song/raw/response2.json", "LhiRts68_bk"),
        ("song/raw/response3.json", "Zi_XLOBDo_Y"),
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
        "account/raw/account_info.json",
        "account_account_menu",
        client.get_account_info(),
    )
    .await?;

    let playlists = capture(
        scratch_dir,
        fixture_root,
        "library/playlists/raw/first_page.json",
        "browse",
        client.get_library_playlists(),
    )
    .await?;
    if let Some(page) = playlists {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/playlists/raw/continuation.json",
                "browse",
                client.get_library_playlists_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/playlists/raw/first_page.json returned no continuation token");
        }
    }

    let artists = capture(
        scratch_dir,
        fixture_root,
        "library/artists/raw/first_page.json",
        "browse",
        client.get_library_artists(),
    )
    .await?;
    if let Some(page) = artists {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/artists/raw/continuation.json",
                "browse",
                client.get_library_artists_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/artists/raw/first_page.json returned no continuation token");
        }
    }

    let albums = capture(
        scratch_dir,
        fixture_root,
        "library/albums/raw/first_page.json",
        "browse",
        client.get_library_albums(),
    )
    .await?;
    if let Some(page) = albums {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/albums/raw/continuation.json",
                "browse",
                client.get_library_albums_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/albums/raw/first_page.json returned no continuation token");
        }
    }

    let subscriptions = capture(
        scratch_dir,
        fixture_root,
        "library/subscriptions/raw/first_page.json",
        "browse",
        client.get_library_subscriptions(),
    )
    .await?;
    if let Some(page) = subscriptions {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/subscriptions/raw/continuation.json",
                "browse",
                client.get_library_subscriptions_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/subscriptions/raw/first_page.json returned no continuation token");
        }
    }

    let channels = capture(
        scratch_dir,
        fixture_root,
        "library/channels/raw/first_page.json",
        "browse",
        client.get_library_channels(),
    )
    .await?;
    if let Some(page) = channels {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/channels/raw/continuation.json",
                "browse",
                client.get_library_channels_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/channels/raw/first_page.json returned no continuation token");
        }
    }

    let podcasts = capture(
        scratch_dir,
        fixture_root,
        "library/podcasts/raw/first_page.json",
        "browse",
        client.get_library_podcasts(),
    )
    .await?;
    if let Some(page) = podcasts {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/podcasts/raw/continuation.json",
                "browse",
                client.get_library_podcasts_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/podcasts/raw/first_page.json returned no continuation token");
        }
    }

    let songs = capture(
        scratch_dir,
        fixture_root,
        "library/songs/raw/first_page.json",
        "browse",
        client.get_library_songs(),
    )
    .await?;
    if let Some(page) = songs {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/songs/raw/continuation.json",
                "browse",
                client.get_library_songs_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/songs/raw/first_page.json returned no continuation token");
        }
    }

    let liked_songs = capture(
        scratch_dir,
        fixture_root,
        "library/liked_songs/raw/first_page.json",
        "browse",
        client.get_liked_songs(),
    )
    .await?;
    if let Some(page) = liked_songs {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/liked_songs/raw/continuation.json",
                "browse",
                client.get_liked_songs_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/liked_songs/raw/first_page.json returned no continuation token");
        }
    }

    let saved_episodes = capture(
        scratch_dir,
        fixture_root,
        "library/saved_episodes/raw/first_page.json",
        "browse",
        client.get_saved_episodes(),
    )
    .await?;
    if let Some(page) = saved_episodes {
        if let Some(token) = page.continuation {
            capture(
                scratch_dir,
                fixture_root,
                "library/saved_episodes/raw/continuation.json",
                "browse",
                client.get_saved_episodes_continuation(token),
            )
            .await?;
        } else {
            eprintln!("library/saved_episodes/raw/first_page.json returned no continuation token");
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
        fs::copy(&captured, &target)?;
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
