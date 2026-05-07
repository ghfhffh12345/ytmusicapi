use serde_json::Value;

use crate::{ArtistRef, Error, LibraryAlbum, LibraryAlbumsContinuationToken, Page};

use super::core::{
    continuation_grid, continuation_grid_items, extract_continuation_token,
    library_grid_continuation, library_grid_items, optional_text, parse_thumbnails, required_text,
};

pub(crate) fn parse_library_albums_response(
    response: &Value,
) -> Result<Page<LibraryAlbum, LibraryAlbumsContinuationToken>, Error> {
    Ok(Page {
        items: library_grid_items(response)?
            .iter()
            .map(parse_library_album)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: library_grid_continuation(response, |token| {
            crate::LibraryAlbumsContinuationToken::new(token)
        })?,
    })
}

pub(crate) fn parse_library_albums_continuation(
    response: &Value,
) -> Result<Page<LibraryAlbum, LibraryAlbumsContinuationToken>, Error> {
    Ok(Page {
        items: continuation_grid_items(response)?
            .iter()
            .map(parse_library_album)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: extract_continuation_token(continuation_grid(response)?, |token| {
            crate::LibraryAlbumsContinuationToken::new(token)
        }),
    })
}

fn parse_library_album(item: &Value) -> Result<LibraryAlbum, Error> {
    let renderer = item.get("musicTwoRowItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicTwoRowItemRenderer in album grid item".to_owned(),
        )
    })?;
    let subtitle_runs = renderer
        .pointer("/subtitle/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let year = parse_year(subtitle_runs);

    Ok(LibraryAlbum {
        browse_id: required_text(
            renderer,
            "/title/runs/0/navigationEndpoint/browseEndpoint/browseId",
        )?,
        playlist_id: parse_playlist_id(renderer),
        title: required_text(renderer, "/title/runs/0/text")?,
        type_label: subtitle_runs
            .first()
            .and_then(|run| optional_text(run, "/text")),
        artists: parse_artists(subtitle_runs, year.as_deref()),
        year,
        thumbnails: parse_thumbnails(renderer)?,
    })
}

fn parse_playlist_id(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/menu/menuRenderer/items/0/menuNavigationItemRenderer/navigationEndpoint/watchPlaylistEndpoint/playlistId",
    )
}

fn parse_artists(runs: &[Value], year: Option<&str>) -> Vec<ArtistRef> {
    metadata_runs(runs)
        .iter()
        .filter_map(|run| {
            let text = optional_text(run, "/text")?;
            let trimmed = text.trim();
            let browse_id = optional_text(run, "/navigationEndpoint/browseEndpoint/browseId");
            if trimmed.is_empty()
                || trimmed == "•"
                || (Some(trimmed) == year && browse_id.is_none())
            {
                return None;
            }

            Some(ArtistRef {
                id: browse_id.unwrap_or_default(),
                name: text,
            })
        })
        .collect()
}

fn parse_year(runs: &[Value]) -> Option<String> {
    metadata_runs(runs).iter().rev().find_map(|run| {
        let text = optional_text(run, "/text")?;
        let trimmed = text.trim();
        if looks_like_year(trimmed)
            && optional_text(run, "/navigationEndpoint/browseEndpoint/browseId").is_none()
        {
            return Some(trimmed.to_owned());
        }

        None
    })
}

fn metadata_runs(runs: &[Value]) -> &[Value] {
    runs.iter()
        .position(is_separator_run)
        .map(|index| &runs[index + 1..])
        .unwrap_or(&[])
}

fn is_separator_run(run: &Value) -> bool {
    optional_text(run, "/text")
        .map(|text| text.trim() == "•")
        .unwrap_or(false)
}

fn looks_like_year(text: &str) -> bool {
    text.len() == 4 && text.chars().all(|ch| ch.is_ascii_digit())
}
