use serde_json::Value;

use crate::{ArtistRef, Error, LibraryPlaylist, Thumbnail};

pub(crate) fn parse_library_playlists_response(
    response: &Value,
) -> Result<Vec<LibraryPlaylist>, Error> {
    let Some(items) = library_playlist_items(response)? else {
        return Ok(Vec::new());
    };

    items
        .iter()
        .filter_map(|item| item.get("musicTwoRowItemRenderer"))
        .map(parse_library_playlist)
        .collect()
}

fn library_playlist_items<'a>(response: &'a Value) -> Result<Option<&'a [Value]>, Error> {
    let tabs = required_array_at(response, "/contents/singleColumnBrowseResultsRenderer/tabs")?;

    let library_tab = tabs
        .iter()
        .find(|tab| {
            tab.pointer("/tabRenderer/selected")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .ok_or_else(|| Error::Parse("library response missing selected library tab".to_owned()))?;

    let sections = required_array_at(
        library_tab,
        "/tabRenderer/content/sectionListRenderer/contents",
    )?;

    for section in sections {
        if let Some(items) = section
            .pointer("/gridRenderer/items")
            .and_then(Value::as_array)
        {
            return Ok(Some(items.as_slice()));
        }
    }

    Ok(None)
}

fn parse_library_playlist(renderer: &Value) -> Result<LibraryPlaylist, Error> {
    let title_run = required_value_at(renderer, "/title/runs/0")?;
    let browse_id = required_text(title_run, "/navigationEndpoint/browseEndpoint/browseId")?;
    let subtitle_runs = renderer
        .pointer("/subtitle/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    Ok(LibraryPlaylist {
        playlist_id: browse_id
            .strip_prefix("VL")
            .unwrap_or(&browse_id)
            .to_owned(),
        title: optional_text(title_run, "/text"),
        authors: parse_authors(subtitle_runs),
        item_count: parse_item_count(subtitle_runs),
        thumbnails: parse_thumbnails(renderer)?,
    })
}

fn parse_authors(runs: &[Value]) -> Vec<ArtistRef> {
    runs.iter()
        .filter_map(|run| {
            let text = optional_text(run, "/text")?;
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "•" || parse_count_text(trimmed).is_some() {
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

fn parse_item_count(runs: &[Value]) -> Option<u32> {
    runs.iter()
        .filter_map(|run| optional_text(run, "/text"))
        .find_map(|text| parse_count_text(text.trim()))
}

fn parse_count_text(text: &str) -> Option<u32> {
    let mut parts = text.split_whitespace();
    let count = parts.next()?.parse().ok()?;
    let unit = parts.next()?.to_ascii_lowercase();
    unit.starts_with("song").then_some(count)
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

fn required_array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], Error> {
    required_value_at(value, pointer)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))
}

fn required_value_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value, Error> {
    value
        .pointer(pointer)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))
}
