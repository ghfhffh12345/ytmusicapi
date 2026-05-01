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
        .skip(1)
        .filter_map(|item| item.get("musicTwoRowItemRenderer"))
        .map(parse_library_playlist)
        .collect()
}

fn library_playlist_items(response: &Value) -> Result<Option<&[Value]>, Error> {
    let tabs = required_array_at(response, "/contents/singleColumnBrowseResultsRenderer/tabs")?;

    let library_tab = tabs
        .iter()
        .find(|tab| is_selected_tab(tab))
        .or_else(|| legacy_library_tab(tabs))
        .ok_or_else(|| Error::Parse("library response missing selected library tab".to_owned()))?;

    let sections = required_array_at(
        library_tab,
        "/tabRenderer/content/sectionListRenderer/contents",
    )?;

    for section in sections {
        if let Some(items) = section_grid_items(section)? {
            return Ok(Some(items));
        }
    }

    Ok(None)
}

fn is_selected_tab(tab: &Value) -> bool {
    tab.pointer("/tabRenderer/selected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn legacy_library_tab(tabs: &[Value]) -> Option<&Value> {
    if tabs
        .iter()
        .any(|tab| tab.pointer("/tabRenderer/selected").is_some())
    {
        return None;
    }

    let library_tab_index = match tabs.len() {
        2 => 1,
        3.. => 2,
        _ => return None,
    };

    tabs.get(library_tab_index)
}

fn section_grid_items(section: &Value) -> Result<Option<&[Value]>, Error> {
    if section.get("gridRenderer").is_some() {
        return required_array_at(section, "/gridRenderer/items").map(Some);
    }

    let Some(contents) = section
        .pointer("/itemSectionRenderer/contents")
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };

    for content in contents {
        if content.get("gridRenderer").is_some() {
            return required_array_at(content, "/gridRenderer/items").map(Some);
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
    let (author_runs, count_runs) = split_subtitle_runs(subtitle_runs);

    Ok(LibraryPlaylist {
        playlist_id: browse_id
            .strip_prefix("VL")
            .unwrap_or(&browse_id)
            .to_owned(),
        title: optional_text(title_run, "/text"),
        authors: parse_authors(author_runs),
        item_count: parse_item_count(count_runs),
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

fn split_subtitle_runs(runs: &[Value]) -> (&[Value], &[Value]) {
    let Some(separator_index) = runs.iter().position(is_separator_run) else {
        return (runs, runs);
    };

    (&runs[..separator_index], &runs[separator_index + 1..])
}

fn is_separator_run(run: &Value) -> bool {
    optional_text(run, "/text")
        .map(|text| text.trim() == "•")
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
