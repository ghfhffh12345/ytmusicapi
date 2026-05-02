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

    Ok(LibrarySong {
        video_id: required_text(renderer, "/playlistItemData/videoId")?,
        title: required_text(
            renderer,
            "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
        )?,
        artists: parse_artists(renderer),
        album: parse_album(renderer),
        duration: parse_duration(renderer),
        thumbnails: parse_thumbnails(renderer)?,
        like_status: parse_like_status(renderer),
    })
}

fn is_leading_random_mix_tile(item: &Value) -> bool {
    item.get("musicResponsiveListItemRenderer")
        .is_some_and(|renderer| optional_text(renderer, "/playlistItemData/videoId").is_none())
}

fn parse_artists(renderer: &Value) -> Vec<ArtistRef> {
    renderer
        .pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter_map(|run| {
            let text = optional_text(run, "/text")?;
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "•" {
                return None;
            }

            Some(ArtistRef {
                id: optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
                    .unwrap_or_default(),
                name: text,
            })
        })
        .collect()
}

fn parse_album(renderer: &Value) -> Option<AlbumRef> {
    let run =
        renderer.pointer("/flexColumns/2/musicResponsiveListItemFlexColumnRenderer/text/runs/0")?;

    Some(AlbumRef {
        id: optional_text(run, "/navigationEndpoint/browseEndpoint/browseId").unwrap_or_default(),
        name: optional_text(run, "/text")?,
    })
}

fn parse_duration(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text",
    )
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
