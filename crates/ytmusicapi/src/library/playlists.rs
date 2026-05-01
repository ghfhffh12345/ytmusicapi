use serde_json::Value;

use crate::{ArtistRef, Error, LibraryPlaylist, Thumbnail};

pub(crate) fn parse_library_playlists_response(
    response: &Value,
) -> Result<Vec<LibraryPlaylist>, Error> {
    let items = super::core::library_grid_items(response)?;

    items
        .iter()
        .enumerate()
        .filter(|(index, item)| !(*index == 0 && is_leading_control_tile(item)))
        .map(|(_, item)| item)
        .map(|item| {
            item.get("musicTwoRowItemRenderer").ok_or_else(|| {
                Error::Parse(
                    "library response missing musicTwoRowItemRenderer in playlist grid item"
                        .to_owned(),
                )
            })
        })
        .map(|renderer| renderer.and_then(parse_library_playlist))
        .collect()
}

fn parse_library_playlist(renderer: &Value) -> Result<LibraryPlaylist, Error> {
    let browse_id = library_playlist_browse_id(renderer)?;
    let subtitle_runs = renderer
        .pointer("/subtitle/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let (author_runs, count_runs) = split_subtitle_runs(subtitle_runs);

    Ok(LibraryPlaylist {
        playlist_id: browse_id
            .strip_prefix("VL")
            .unwrap_or(&browse_id)
            .to_owned(),
        title: library_playlist_title(renderer),
        authors: parse_authors(author_runs),
        item_count: parse_item_count(count_runs),
        thumbnails: parse_thumbnails(renderer)?,
    })
}

fn is_leading_control_tile(item: &Value) -> bool {
    item.get("musicTwoRowItemRenderer")
        .is_some_and(|renderer| optional_library_playlist_browse_id(renderer).is_none())
}

fn library_playlist_browse_id(renderer: &Value) -> Result<String, Error> {
    optional_library_playlist_browse_id(renderer)
        .ok_or_else(|| Error::Parse("library response missing playlist browse id".to_owned()))
}

fn optional_library_playlist_browse_id(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/title/runs/0/navigationEndpoint/browseEndpoint/browseId",
    )
    .or_else(|| optional_text(renderer, "/navigationEndpoint/browseEndpoint/browseId"))
}

fn library_playlist_title(renderer: &Value) -> Option<String> {
    optional_text(renderer, "/title/runs/0/text")
        .or_else(|| optional_text(renderer, "/title/simpleText"))
}

fn parse_authors(runs: &[Value]) -> Vec<ArtistRef> {
    runs.iter()
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

fn split_subtitle_runs(runs: &[Value]) -> (&[Value], &[Value]) {
    if let Some(separator_index) = runs.iter().position(is_separator_run) {
        return (&runs[..separator_index], &runs[separator_index + 1..]);
    }

    if runs.len() > 1 {
        let (author_runs, trailing_runs) = runs.split_at(runs.len() - 1);
        let trailing_count = trailing_runs
            .first()
            .and_then(|run| optional_text(run, "/text"))
            .and_then(|text| parse_count_text(text.trim()));

        if trailing_count.is_some() && author_runs.iter().any(has_non_separator_text) {
            return (author_runs, trailing_runs);
        }
    }

    (runs, &[])
}

fn is_separator_run(run: &Value) -> bool {
    optional_text(run, "/text")
        .map(|text| text.trim() == "•")
        .unwrap_or(false)
}

fn has_non_separator_text(run: &Value) -> bool {
    optional_text(run, "/text")
        .map(|text| {
            let trimmed = text.trim();
            !trimmed.is_empty() && trimmed != "•"
        })
        .unwrap_or(false)
}

fn parse_item_count(runs: &[Value]) -> Option<u32> {
    runs.iter()
        .filter_map(|run| optional_text(run, "/text"))
        .find_map(|text| parse_count_text(text.trim()))
}

fn parse_count_text(text: &str) -> Option<u32> {
    let digits_end = text
        .find(|ch: char| !ch.is_ascii_digit() && ch != ',')
        .unwrap_or(text.len());
    if digits_end == 0 {
        return None;
    }

    if text[digits_end..]
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric())
    {
        return None;
    }

    text[..digits_end].replace(',', "").parse().ok()
}

fn parse_thumbnails(value: &Value) -> Result<Vec<Thumbnail>, Error> {
    let Some(thumbnails) = value
        .pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(Value::as_array)
    else {
        return Ok(Vec::new());
    };

    thumbnails
        .iter()
        .map(|thumbnail| {
            Ok(Thumbnail {
                url: required_text(thumbnail, "/url")?,
                width: required_u32(thumbnail, "/width")?,
                height: required_u32(thumbnail, "/height")?,
            })
        })
        .collect()
}

fn optional_text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn required_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))
}

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("library response missing {pointer}")))
}
