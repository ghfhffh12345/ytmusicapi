use serde_json::Value;

use crate::{AlbumRef, ArtistRef, Error, LibraryLikeStatus, LibrarySong};

use super::core::{library_shelf_contents, optional_text, parse_thumbnails, required_text};

pub(crate) fn parse_library_songs_response(response: &Value) -> Result<Vec<LibrarySong>, Error> {
    library_shelf_contents(response)?
        .iter()
        .enumerate()
        .filter(|(index, item)| !(*index == 0 && is_leading_random_mix_tile(item)))
        .map(|(_, item)| parse_library_song(item))
        .collect()
}

fn parse_library_song(item: &Value) -> Result<LibrarySong, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicResponsiveListItemRenderer in song shelf item"
                .to_owned(),
        )
    })?;
    let (title_column_index, title) = parse_title(renderer)?;
    let metadata = parse_song_metadata(renderer, title_column_index);

    Ok(LibrarySong {
        video_id: required_text(renderer, "/playlistItemData/videoId")?,
        title,
        artists: metadata.artists,
        album: metadata.album,
        duration: parse_fixed_duration(renderer).or(metadata.duration),
        thumbnails: parse_thumbnails(renderer)?,
        like_status: parse_like_status(renderer),
    })
}

fn is_leading_random_mix_tile(item: &Value) -> bool {
    item.get("musicResponsiveListItemRenderer")
        .is_some_and(|renderer| {
            renderer.get("playlistItemData").is_none()
                && renderer
                    .pointer("/flexColumns")
                    .and_then(Value::as_array)
                    .is_some_and(|columns| columns.len() == 1)
        })
}

fn parse_title(renderer: &Value) -> Result<(usize, String), Error> {
    flex_columns(renderer)
        .iter()
        .enumerate()
        .find_map(|(index, column)| first_column_text(column).map(|text| (index, text)))
        .ok_or_else(|| Error::Parse("library response missing song title".to_owned()))
}

struct ParsedSongMetadata {
    artists: Vec<ArtistRef>,
    album: Option<AlbumRef>,
    duration: Option<String>,
}

fn parse_song_metadata(renderer: &Value, title_column_index: usize) -> ParsedSongMetadata {
    let mut parsed = ParsedSongMetadata {
        artists: Vec::new(),
        album: None,
        duration: None,
    };

    for (index, column) in flex_columns(renderer).iter().enumerate() {
        if index == title_column_index {
            continue;
        }

        for run in flex_column_runs(column) {
            let Some(text) = optional_text(run, "/text") else {
                continue;
            };
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "•" {
                continue;
            }

            if let Some(browse_id) =
                optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
            {
                if parsed.album.is_none() && is_album_run(run, &browse_id) {
                    parsed.album = Some(AlbumRef {
                        id: browse_id,
                        name: text,
                    });
                } else {
                    parsed.artists.push(ArtistRef {
                        id: browse_id,
                        name: text,
                    });
                }
                continue;
            }

            if parsed.duration.is_none() && looks_like_duration(trimmed) {
                parsed.duration = Some(trimmed.to_owned());
            }
        }
    }

    parsed
}

fn parse_fixed_duration(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text",
    )
}

fn flex_columns(renderer: &Value) -> &[Value] {
    renderer
        .pointer("/flexColumns")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn first_column_text(column: &Value) -> Option<String> {
    optional_text(
        column,
        "/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
    )
}

fn flex_column_runs(column: &Value) -> &[Value] {
    column
        .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn is_album_run(run: &Value, browse_id: &str) -> bool {
    matches!(
        optional_text(
            run,
            "/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType",
        )
        .as_deref(),
        Some("MUSIC_PAGE_TYPE_ALBUM") | Some("MUSIC_PAGE_TYPE_AUDIOBOOK")
    ) || browse_id.starts_with("MPRE")
        || browse_id.contains("release_detail")
}

fn looks_like_duration(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty() || !first.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let mut count = 1;
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        count += 1;
    }

    count >= 2
}

fn parse_like_status(renderer: &Value) -> Option<LibraryLikeStatus> {
    match optional_text(
        renderer,
        "/menu/menuRenderer/topLevelButtons/0/likeButtonRenderer/likeStatus",
    )
    .as_deref()
    {
        Some("LIKE") => Some(LibraryLikeStatus::Like),
        Some("INDIFFERENT") => Some(LibraryLikeStatus::Indifferent),
        Some("DISLIKE") => Some(LibraryLikeStatus::Dislike),
        _ => None,
    }
}
