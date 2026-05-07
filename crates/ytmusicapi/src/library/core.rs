use serde_json::Value;

use crate::{Error, Thumbnail};

pub(crate) struct ArtistLikeRow {
    pub(crate) browse_id: String,
    pub(crate) name: String,
    pub(crate) subscribers: Option<String>,
    pub(crate) thumbnails: Vec<Thumbnail>,
}

pub(crate) fn selected_library_tab(response: &Value) -> Result<&Value, Error> {
    let tabs = required_array_at(response, "/contents/singleColumnBrowseResultsRenderer/tabs")?;

    tabs.iter()
        .find(|tab| is_selected_tab(tab))
        .or_else(|| legacy_library_tab(tabs))
        .ok_or_else(|| Error::Parse("library response missing selected library tab".to_owned()))
}

pub(crate) fn library_grid_items(response: &Value) -> Result<&[Value], Error> {
    let library_tab = selected_library_tab(response)?;
    let sections = required_array_at(
        library_tab,
        "/tabRenderer/content/sectionListRenderer/contents",
    )?;
    let mut saw_empty_library_message = false;

    for section in sections {
        if let Some(items) = section_grid_items(section)? {
            return Ok(items);
        }

        saw_empty_library_message |= section_empty_library_message(section);
    }

    if saw_empty_library_message {
        return Ok(&[]);
    }

    Err(Error::Parse(
        "library response missing grid items in selected library tab".to_owned(),
    ))
}

pub(crate) fn library_grid_continuation<C>(
    response: &Value,
    make_token: impl FnOnce(&str) -> C + Copy,
) -> Result<Option<C>, Error> {
    let library_tab = selected_library_tab(response)?;
    let sections = required_array_at(library_tab, "/content/sectionListRenderer/contents")?;

    for section in sections {
        let Some(renderer) = section.get("gridRenderer") else {
            continue;
        };

        if let Some(token) = extract_continuation_token(renderer, make_token) {
            return Ok(Some(token));
        }
    }

    Ok(None)
}

pub(crate) fn library_shelf_contents(response: &Value) -> Result<&[Value], Error> {
    let library_tab = selected_library_tab(response)?;
    let sections = required_array_at(
        library_tab,
        "/tabRenderer/content/sectionListRenderer/contents",
    )?;
    let mut saw_empty_library_message = false;

    for section in sections {
        if let Some(contents) = section_shelf_contents(section)? {
            return Ok(contents);
        }

        saw_empty_library_message |= section_empty_library_message(section);
    }

    if saw_empty_library_message {
        return Ok(&[]);
    }

    Err(Error::Parse(
        "library response missing shelf contents in selected library tab".to_owned(),
    ))
}

pub(crate) fn library_shelf_continuation<C>(
    response: &Value,
    make_token: impl FnOnce(&str) -> C + Copy,
) -> Result<Option<C>, Error> {
    let library_tab = selected_library_tab(response)?;
    let sections = required_array_at(library_tab, "/content/sectionListRenderer/contents")?;

    for section in sections {
        let Some(renderer) = section.get("musicShelfRenderer") else {
            continue;
        };

        if let Some(token) = extract_continuation_token(renderer, make_token) {
            return Ok(Some(token));
        }
    }

    Ok(None)
}

pub(crate) fn extract_continuation_token<C>(
    value: &Value,
    make_token: impl FnOnce(&str) -> C,
) -> Option<C> {
    optional_text(value, "/continuations/0/nextContinuationData/continuation")
        .as_deref()
        .map(make_token)
}

pub(crate) fn continuation_grid(response: &Value) -> Result<&Value, Error> {
    required_value_at(response, "/continuationContents/gridContinuation")
}

pub(crate) fn continuation_grid_items(response: &Value) -> Result<&[Value], Error> {
    required_array_at(response, "/continuationContents/gridContinuation/items")
}

pub(crate) fn continuation_shelf(response: &Value) -> Result<&Value, Error> {
    required_value_at(response, "/continuationContents/musicShelfContinuation")
}

pub(crate) fn continuation_shelf_contents(response: &Value) -> Result<&[Value], Error> {
    required_array_at(
        response,
        "/continuationContents/musicShelfContinuation/contents",
    )
}

pub(crate) fn parse_thumbnails(value: &Value) -> Result<Vec<Thumbnail>, Error> {
    let thumbnails = value
        .pointer("/thumbnailRenderer/musicThumbnailRenderer/thumbnail/thumbnails")
        .and_then(Value::as_array)
        .or_else(|| {
            value
                .pointer("/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails")
                .and_then(Value::as_array)
        });
    let Some(thumbnails) = thumbnails else {
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

pub(crate) fn optional_text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

pub(crate) fn required_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))
}

pub(crate) fn optional_runs_text(value: &Value, pointer: &str) -> Option<String> {
    let runs = value.pointer(pointer)?.as_array()?;
    let mut text = String::new();
    for run in runs {
        text.push_str(run.pointer("/text").and_then(Value::as_str)?);
    }

    Some(text)
}

pub(crate) fn required_runs_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_runs_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))
}

pub(crate) fn parse_artist_like_row(
    item: &Value,
    item_label: &str,
) -> Result<ArtistLikeRow, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(format!(
            "library response missing musicResponsiveListItemRenderer in {item_label}"
        ))
    })?;

    Ok(ArtistLikeRow {
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        name: required_runs_text(
            renderer,
            "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs",
        )?,
        subscribers: optional_runs_text(
            renderer,
            "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs",
        )
        .and_then(first_token),
        thumbnails: parse_thumbnails(renderer)?,
    })
}

pub(crate) fn first_token(value: String) -> Option<String> {
    value.split(' ').next().map(str::to_owned)
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

fn section_grid_renderer(section: &Value) -> Option<&Value> {
    if let Some(renderer) = section.get("gridRenderer") {
        return Some(renderer);
    }

    let Some(contents) = section
        .pointer("/itemSectionRenderer/contents")
        .and_then(Value::as_array)
    else {
        return None;
    };

    for content in contents {
        if let Some(renderer) = content.get("gridRenderer") {
            return Some(renderer);
        }
    }

    None
}

fn section_grid_items(section: &Value) -> Result<Option<&[Value]>, Error> {
    section_grid_renderer(section)
        .map(|renderer| required_array_at(renderer, "/items"))
        .transpose()
}

fn section_shelf_renderer(section: &Value) -> Option<&Value> {
    if let Some(renderer) = section.get("musicShelfRenderer") {
        return Some(renderer);
    }

    let Some(contents) = section
        .pointer("/itemSectionRenderer/contents")
        .and_then(Value::as_array)
    else {
        return None;
    };

    for content in contents {
        if let Some(renderer) = content.get("musicShelfRenderer") {
            return Some(renderer);
        }
    }

    None
}

fn section_shelf_contents(section: &Value) -> Result<Option<&[Value]>, Error> {
    section_shelf_renderer(section)
        .map(|renderer| required_array_at(renderer, "/contents"))
        .transpose()
}

pub(crate) fn section_empty_library_message(section: &Value) -> bool {
    let Some(contents) = section
        .pointer("/itemSectionRenderer/contents")
        .and_then(Value::as_array)
    else {
        return false;
    };

    !contents.is_empty() && contents.iter().all(is_known_empty_library_message)
}

pub(crate) fn section_message_only_without_subtext(section: &Value) -> bool {
    let Some(contents) = section
        .pointer("/itemSectionRenderer/contents")
        .and_then(Value::as_array)
    else {
        return false;
    };

    !contents.is_empty() && contents.iter().all(is_message_only_without_subtext)
}

fn is_known_empty_library_message(content: &Value) -> bool {
    let Some(message) = content.get("messageRenderer") else {
        return false;
    };

    matches!(
        (
            message_text(message),
            message_subtext(message),
        ),
        (
            Some(text),
            Some(subtext),
        ) if matches!(
            (text.as_str(), subtext.as_str()),
            ("No playlists saved yet", "Your saved playlists will show up here")
                | ("No songs yet", "Songs you save to your library will show here")
                | ("No albums yet", "Albums you save to your library will show here")
                | ("아직 아티스트가 없습니다", "저장한 음악의 아티스트가 여기에 표시됩니다")
                | (
                    "No subscriptions yet",
                    "Channels you subscribe to will show here"
                )
        )
    )
}

fn message_text(message: &Value) -> Option<String> {
    optional_runs_text(message, "/text/runs").or_else(|| optional_text(message, "/text/simpleText"))
}

fn message_subtext(message: &Value) -> Option<String> {
    optional_runs_text(message, "/subtext/messageSubtextRenderer/text/runs")
        .or_else(|| optional_text(message, "/subtext/messageSubtextRenderer/text/simpleText"))
}

fn is_message_only_without_subtext(content: &Value) -> bool {
    let Some(message) = content.get("messageRenderer") else {
        return false;
    };

    message_text(message).is_some() && message_subtext(message).is_none()
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

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("library response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("library response missing {pointer}")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{library_grid_items, library_shelf_contents, selected_library_tab};
    use crate::Error;

    #[test]
    fn selected_library_tab_prefers_selected_marker() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicShelfRenderer": {
                                            "contents": []
                                        }
                                    }]
                                }
                            }
                        }
                    }, {
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "gridRenderer": {
                                            "items": []
                                        }
                                    }]
                                }
                            }
                        }
                    }, {
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "gridRenderer": {
                                            "items": [{
                                                "musicTwoRowItemRenderer": {
                                                    "title": {
                                                        "runs": [{
                                                            "text": "Fallback Grid Playlist"
                                                        }]
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let selected_tab = selected_library_tab(&response).unwrap();

        assert_eq!(
            selected_tab
                .pointer("/tabRenderer/selected")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(
            selected_tab
                .pointer("/tabRenderer/content/sectionListRenderer/contents/0/gridRenderer")
                .is_none()
        );
    }

    #[test]
    fn library_grid_items_errors_when_selected_tab_has_no_grid() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "gridRenderer": {
                                            "items": [{
                                                "musicTwoRowItemRenderer": {
                                                    "title": {
                                                        "runs": [{
                                                            "text": "Wrong Tab Playlist"
                                                        }]
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }, {
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicShelfRenderer": {
                                            "contents": []
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let error = library_grid_items(&response).unwrap_err();

        assert!(matches!(error, crate::Error::Parse(_)));
    }

    #[test]
    fn library_shelf_contents_returns_selected_tab_shelf() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "musicShelfRenderer": {
                                            "contents": [{
                                                "musicResponsiveListItemRenderer": {
                                                    "navigationEndpoint": {
                                                        "browseEndpoint": {
                                                            "browseId": "UCArtist1"
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let contents = library_shelf_contents(&response).unwrap();

        assert_eq!(contents.len(), 1);
    }

    #[test]
    fn library_shelf_contents_errors_for_generic_message_only_section() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "messageRenderer": {
                                                    "text": {
                                                        "runs": [{
                                                            "text": "Something went wrong"
                                                        }]
                                                    },
                                                    "subtext": {
                                                        "messageSubtextRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Try again later"
                                                                }]
                                                            }
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let error = library_shelf_contents(&response).unwrap_err();
        assert!(matches!(
            error,
            Error::Parse(message)
                if message == "library response missing shelf contents in selected library tab"
        ));
    }

    #[test]
    fn library_grid_items_returns_empty_for_known_empty_library_message() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                            "itemSectionRenderer": {
                                                "contents": [{
                                                    "messageRenderer": {
                                                        "text": {
                                                            "runs": [{
                                                                "text": "No playlists saved yet"
                                                            }]
                                                        },
                                                        "subtext": {
                                                            "messageSubtextRenderer": {
                                                                "text": {
                                                                    "runs": [{
                                                                        "text": "Your saved playlists will show up here"
                                                                    }]
                                                                }
                                                            }
                                                        }
                                                    }
                                                }]
                                            }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let items = library_grid_items(&response).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn library_grid_items_errors_for_generic_message_only_section() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "messageRenderer": {
                                                    "text": {
                                                        "runs": [{
                                                            "text": "Something went wrong"
                                                        }]
                                                    },
                                                    "subtext": {
                                                        "messageSubtextRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Try again later"
                                                                }]
                                                            }
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let error = library_grid_items(&response).unwrap_err();
        assert!(matches!(
            error,
            Error::Parse(message)
                if message == "library response missing grid items in selected library tab"
        ));
    }

    #[test]
    fn library_shelf_contents_errors_for_alternate_generic_message_only_section() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "messageRenderer": {
                                                    "text": {
                                                        "runs": [{
                                                            "text": "Temporarily unavailable"
                                                        }]
                                                    },
                                                    "subtext": {
                                                        "messageSubtextRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Please refresh and try again"
                                                                }]
                                                            }
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let error = library_shelf_contents(&response).unwrap_err();
        assert!(matches!(
            error,
            Error::Parse(message)
                if message == "library response missing shelf contents in selected library tab"
        ));
    }

    #[test]
    fn library_grid_items_returns_empty_for_known_empty_subscriptions_message() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                            "itemSectionRenderer": {
                                                "contents": [{
                                                    "messageRenderer": {
                                                        "text": {
                                                            "simpleText": "No subscriptions yet"
                                                        },
                                                        "subtext": {
                                                            "messageSubtextRenderer": {
                                                                "text": {
                                                                    "simpleText": "Channels you subscribe to will show here"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }]
                                            }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let items = library_grid_items(&response).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn library_shelf_contents_returns_empty_for_known_empty_subscriptions_runs_message() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "messageRenderer": {
                                                    "text": {
                                                        "runs": [{
                                                            "text": "No subscriptions yet"
                                                        }]
                                                    },
                                                    "subtext": {
                                                        "messageSubtextRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Channels you subscribe to will show here"
                                                                }]
                                                            }
                                                        }
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let contents = library_shelf_contents(&response).unwrap();
        assert!(contents.is_empty());
    }

    #[test]
    fn library_grid_items_errors_for_generic_simple_text_message_only_section() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                        "itemSectionRenderer": {
                                            "contents": [{
                                                "messageRenderer": {
                                                    "text": {
                                                        "simpleText": "Something went wrong"
                                                    }
                                                }
                                            }]
                                        }
                                    }]
                                }
                            }
                        }
                    }]
                }
            }
        });

        let error = library_grid_items(&response).unwrap_err();
        assert!(matches!(
            error,
            Error::Parse(message)
                if message == "library response missing grid items in selected library tab"
        ));
    }
}
