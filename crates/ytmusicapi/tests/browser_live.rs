use std::path::PathBuf;

use ytmusicapi::{SearchFilter, SearchQuery, YtMusic};

#[tokio::test]
#[ignore = "requires local browser.json generated from browser.txt and live network access"]
async fn get_library_playlists_live_smoke_test() {
    let worktree_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let repo_root = worktree_root
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == ".worktrees"))
        .and_then(|worktrees_dir| worktrees_dir.parent())
        .map(|shared_root| shared_root.to_path_buf())
        .unwrap_or(worktree_root);
    let browser_json = repo_root.join("browser.json");

    assert!(
        browser_json.exists(),
        "run `cargo run -p ytmusicapi-cli < browser.txt` from the repo root first"
    );

    let client = YtMusic::from_browser_auth_file(&browser_json).unwrap();
    let playlists = client.get_library_playlists().await.unwrap();
    assert!(
        !playlists.items.is_empty(),
        "expected at least one library playlist from the authenticated account"
    );
    if let Some(token) = playlists.continuation.clone() {
        let continuation = client
            .get_library_playlists_continuation(token)
            .await
            .unwrap();
        assert!(
            continuation
                .items
                .iter()
                .all(|playlist| !playlist.playlist_id.is_empty()),
            "expected each live playlist continuation item to include stable identity fields"
        );
    } else {
        eprintln!("library playlists returned no continuation token for this account");
    }

    let artists = client.get_library_artists().await.unwrap();
    if artists.items.is_empty() {
        eprintln!(
            "library artists returned 0 items for this account; verified empty-state parsing"
        );
    }

    let albums = client.get_library_albums().await.unwrap();
    if albums.items.is_empty() {
        eprintln!("library albums returned 0 items for this account; verified empty-state parsing");
    }

    let songs = client.get_library_songs().await.unwrap();
    if songs.items.is_empty() {
        eprintln!("library songs returned 0 items for this account; verified empty-state parsing");
    } else {
        assert!(
            songs
                .items
                .iter()
                .all(|song| !song.video_id.is_empty() && !song.title.is_empty()),
            "expected each live library song to include stable identity fields"
        );
    }
    if let Some(token) = songs.continuation.clone() {
        let continuation = client.get_library_songs_continuation(token).await.unwrap();
        assert!(
            continuation
                .items
                .iter()
                .all(|song| !song.video_id.is_empty() && !song.title.is_empty()),
            "expected each live song continuation item to include stable identity fields"
        );
    } else {
        eprintln!("library songs returned no continuation token for this account");
    }

    let subscriptions = client.get_library_subscriptions().await.unwrap();
    if subscriptions.items.is_empty() {
        eprintln!(
            "library subscriptions returned 0 items for this account; verified empty-state parsing"
        );
    } else {
        assert!(
            subscriptions
                .items
                .iter()
                .all(|subscription| !subscription.browse_id.is_empty()
                    && !subscription.name.is_empty()),
            "expected each live library subscription to include stable identity fields"
        );
    }

    let channels = client.get_library_channels().await.unwrap();
    if channels.items.is_empty() {
        eprintln!(
            "library channels returned 0 items for this account; verified empty-state parsing"
        );
    } else {
        assert!(
            channels
                .items
                .iter()
                .all(|channel| !channel.browse_id.is_empty() && !channel.name.is_empty()),
            "expected each live library channel to include stable identity fields"
        );
    }

    let podcasts = client.get_library_podcasts().await.unwrap();
    if podcasts.items.is_empty() {
        eprintln!(
            "library podcasts returned 0 items for this account; verified empty-state parsing"
        );
    } else {
        assert!(
            podcasts.items.iter().all(|podcast| {
                !podcast.title.is_empty()
                    && !podcast.browse_id.is_empty()
                    && !podcast.podcast_id.is_empty()
                    && !podcast.channel.name.is_empty()
            }),
            "expected each live library podcast to include stable identity and display fields"
        );
    }

    let liked_songs = client.get_liked_songs().await.unwrap();
    assert!(
        !liked_songs.playlist_id.is_empty() && !liked_songs.title.is_empty(),
        "expected live liked songs metadata to include stable identity fields"
    );
    if liked_songs.items.is_empty() {
        eprintln!("liked songs returned 0 items for this account; verified empty-state parsing");
    } else {
        assert!(
            liked_songs
                .items
                .iter()
                .all(|song| !song.video_id.is_empty() && !song.title.is_empty()),
            "expected each live liked song to include stable identity fields"
        );
    }
    if let Some(token) = liked_songs.continuation.clone() {
        let continuation = client.get_liked_songs_continuation(token).await.unwrap();
        assert!(
            !continuation.playlist_id.is_empty() && !continuation.title.is_empty(),
            "expected liked songs continuation to include wrapper metadata"
        );
        if continuation.items.is_empty() {
            eprintln!(
                "liked songs continuation returned 0 items for this account; verified empty-state parsing"
            );
        } else {
            assert!(
                continuation
                    .items
                    .iter()
                    .all(|song| !song.video_id.is_empty() && !song.title.is_empty()),
                "expected each live liked song continuation item to include stable identity fields"
            );
        }
    } else {
        eprintln!("liked songs returned no continuation token for this account");
    }

    let saved_episodes = client.get_saved_episodes().await.unwrap();
    assert!(
        !saved_episodes.playlist_id.is_empty() && !saved_episodes.title.is_empty(),
        "expected live saved episodes metadata to include stable identity fields"
    );
    if saved_episodes.items.is_empty() {
        eprintln!("saved episodes returned 0 items for this account; verified empty-state parsing");
    } else {
        assert!(
            saved_episodes.items.iter().all(|episode| {
                !episode.video_id.is_empty()
                    && !episode.title.is_empty()
                    && !episode.channel.is_empty()
                    && !episode.podcast.is_empty()
            }),
            "expected each live saved episode to include stable identity and display fields"
        );
    }

    let account_info = client.get_account_info().await.unwrap();
    assert!(
        !account_info.account_name.is_empty() && !account_info.account_photo_url.is_empty(),
        "expected live account info to include account name and photo URL"
    );

    let default_page = client.search(SearchQuery::new("abba")).await.unwrap();
    if let Some(token) = default_page.continuation.clone() {
        let continuation = client.search_continuation(token).await.unwrap();
        if continuation.items.is_empty() {
            eprintln!(
                "search continuation returned 0 items for this account; verified empty-state parsing"
            );
        } else {
            assert!(
                continuation.items.iter().all(|item| matches!(
                    item,
                    ytmusicapi::SearchResult::Song(_)
                        | ytmusicapi::SearchResult::Video(_)
                        | ytmusicapi::SearchResult::Album(_)
                        | ytmusicapi::SearchResult::Artist(_)
                        | ytmusicapi::SearchResult::Playlist(_)
                        | ytmusicapi::SearchResult::Profile(_)
                        | ytmusicapi::SearchResult::Episode(_)
                        | ytmusicapi::SearchResult::Podcast(_)
                )),
                "expected continuation search results to preserve typed variants"
            );
        }
    } else {
        eprintln!("search returned no continuation token for this account");
    }

    let songs = client
        .search(SearchQuery::new("abba").with_filter(SearchFilter::Songs))
        .await
        .unwrap();
    assert!(
        !songs.items.is_empty(),
        "expected authenticated filtered songs results for query `abba`"
    );
    assert!(
        songs
            .items
            .iter()
            .all(|result| matches!(result, ytmusicapi::SearchResult::Song(_))),
        "expected filtered songs search results to contain only songs"
    );
    assert!(
        songs.items.iter().any(|result| match result {
            ytmusicapi::SearchResult::Song(song) => song.album.is_some(),
            _ => false,
        }),
        "expected at least one filtered song result to include album metadata"
    );
    if let Some(token) = songs.continuation.clone() {
        let continuation = client.search_continuation(token).await.unwrap();
        if continuation.items.is_empty() {
            eprintln!(
                "filtered songs continuation returned 0 items for this account; verified empty-state parsing"
            );
        } else {
            assert!(
                continuation
                    .items
                    .iter()
                    .all(|item| matches!(item, ytmusicapi::SearchResult::Song(_))),
                "expected filtered song continuation results to remain song-only"
            );
        }
    } else {
        eprintln!("filtered songs search returned no continuation token for this account");
    }

    let videos = client
        .search(SearchQuery::new("abba").with_filter(SearchFilter::Videos))
        .await
        .unwrap();
    assert!(
        !videos.items.is_empty(),
        "expected authenticated filtered videos results for query `abba`"
    );
    assert!(
        videos
            .items
            .iter()
            .all(|result| matches!(result, ytmusicapi::SearchResult::Video(_))),
        "expected filtered videos search results to contain only videos"
    );
    assert!(
        videos.items.iter().any(|result| match result {
            ytmusicapi::SearchResult::Video(video) => video.views.is_some(),
            _ => false,
        }),
        "expected at least one filtered video result to include view metadata"
    );
    assert!(
        videos.items.iter().any(|result| match result {
            ytmusicapi::SearchResult::Video(video) => video.duration.is_some(),
            _ => false,
        }),
        "expected at least one filtered video result to include duration metadata"
    );
}

#[tokio::test]
#[ignore = "requires local browser.json generated from browser.txt and live network access"]
async fn get_watch_playlist_live_smoke_test() {
    let worktree_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let repo_root = worktree_root
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == ".worktrees"))
        .and_then(|worktrees_dir| worktrees_dir.parent())
        .map(|shared_root| shared_root.to_path_buf())
        .unwrap_or(worktree_root);
    let browser_json = repo_root.join("browser.json");

    assert!(
        browser_json.exists(),
        "run `cargo run -p ytmusicapi-cli < browser.txt` from the repo root first"
    );

    let client = YtMusic::from_browser_auth_file(&browser_json).unwrap();
    let page = client
        .get_watch_playlist(ytmusicapi::WatchPlaylistQuery::new().with_video_id("4y33h81phKU"))
        .await
        .unwrap();

    assert!(
        !page.items.is_empty(),
        "expected live watch playlist to return at least one playable item"
    );
    assert!(
        page.items
            .iter()
            .all(|track| !track.video_id.is_empty() && !track.title.is_empty()),
        "expected each live watch item to include stable identity fields"
    );

    if let Some(token) = page.continuation.clone() {
        let continuation = client.get_watch_playlist_continuation(token).await.unwrap();
        if continuation.items.is_empty() {
            eprintln!("watch continuation returned 0 items; verified empty-state parsing");
        } else {
            assert!(
                continuation
                    .items
                    .iter()
                    .all(|track| !track.video_id.is_empty() && !track.title.is_empty()),
                "expected each live watch continuation item to include stable identity fields"
            );
        }
    } else {
        eprintln!("watch playlist returned no continuation token for this seed");
    }
}
