use serde_json::Value;

use crate::{AlbumRef, ArtistRef, Error, LibraryLikeStatus, LikedSongItem, LikedSongs};

use super::core::{optional_text, parse_thumbnails, required_runs_text, required_text};

pub(crate) fn parse_liked_songs_response(response: &Value) -> Result<LikedSongs, Error> {
    let header = required_value_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicResponsiveHeaderRenderer",
        "library response missing liked songs header",
    )?;
    let items = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/1/musicPlaylistShelfRenderer/contents",
        "library response missing liked songs items",
    )?;

    Ok(LikedSongs {
        playlist_id: "LM".to_owned(),
        title: required_runs_text(header, "/title/runs")?,
        items: items
            .iter()
            .map(parse_liked_song_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: parse_thumbnails(header)?,
    })
}

fn parse_liked_song_item(item: &Value) -> Result<LikedSongItem, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicResponsiveListItemRenderer in liked songs item"
                .to_owned(),
        )
    })?;
    let title = required_title(renderer)?;
    let metadata = parse_song_metadata(renderer);

    Ok(LikedSongItem {
        video_id: required_text(renderer, "/playlistItemData/videoId")?,
        title,
        artists: metadata.artists,
        album: metadata.album,
        duration: parse_fixed_duration(renderer).or(metadata.duration),
        thumbnails: parse_thumbnails(renderer)?,
        like_status: parse_like_status(renderer),
    })
}

struct ParsedSongMetadata {
    artists: Vec<ArtistRef>,
    album: Option<AlbumRef>,
    duration: Option<String>,
}

fn parse_song_metadata(renderer: &Value) -> ParsedSongMetadata {
    let mut parsed = ParsedSongMetadata {
        artists: Vec::new(),
        album: None,
        duration: None,
    };

    for column in flex_columns(renderer).iter().skip(1) {
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
                if parsed.album.is_none() && is_album_browse_id(&browse_id) {
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

fn required_title(renderer: &Value) -> Result<String, Error> {
    flex_columns(renderer)
        .first()
        .and_then(column_title_text)
        .ok_or_else(|| Error::Parse("library response missing liked song title".to_owned()))
}

fn parse_fixed_duration(renderer: &Value) -> Option<String> {
    renderer
        .pointer("/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .and_then(|runs| runs.iter().find_map(|run| optional_text(run, "/text")))
        .map(|text| text.trim().to_owned())
        .filter(|text| looks_like_duration(text))
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

fn required_value_at<'a>(
    value: &'a Value,
    pointer: &str,
    message: &str,
) -> Result<&'a Value, Error> {
    value
        .pointer(pointer)
        .ok_or_else(|| Error::Parse(message.to_owned()))
}

fn required_array_at<'a>(
    value: &'a Value,
    pointer: &str,
    message: &str,
) -> Result<&'a [Value], Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(message.to_owned()))
}

fn flex_columns(renderer: &Value) -> &[Value] {
    renderer
        .get("flexColumns")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn flex_column_runs(column: &Value) -> &[Value] {
    column
        .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn column_title_text(column: &Value) -> Option<String> {
    flex_column_runs(column).iter().find_map(|run| {
        let text = optional_text(run, "/text")?;
        let has_watch_endpoint = run.pointer("/navigationEndpoint/watchEndpoint").is_some();
        has_watch_endpoint.then_some(text)
    })
}

fn is_album_browse_id(browse_id: &str) -> bool {
    browse_id.starts_with("MPRE") || browse_id.starts_with("OLAK")
}

fn looks_like_duration(text: &str) -> bool {
    let parts = text.split(':').collect::<Vec<_>>();
    if !(parts.len() == 2 || parts.len() == 3) {
        return false;
    }

    parts.iter().enumerate().all(|(index, part)| {
        !part.is_empty()
            && part.chars().all(|ch| ch.is_ascii_digit())
            && (index == 0 || part.len() == 2)
    })
}
