use serde_json::Value;

use crate::Error;

pub(crate) fn selected_library_tab<'a>(response: &'a Value) -> Result<&'a Value, Error> {
    let tabs = required_array_at(response, "/contents/singleColumnBrowseResultsRenderer/tabs")?;

    tabs.iter()
        .find(|tab| is_selected_tab(tab))
        .or_else(|| legacy_library_tab(tabs))
        .ok_or_else(|| Error::Parse("library response missing selected library tab".to_owned()))
}

pub(crate) fn library_grid_items<'a>(response: &'a Value) -> Result<&'a [Value], Error> {
    let library_tab = selected_library_tab(response)?;
    let sections = required_array_at(
        library_tab,
        "/tabRenderer/content/sectionListRenderer/contents",
    )?;

    for section in sections {
        if let Some(items) = section_grid_items(section)? {
            return Ok(items);
        }
    }

    Err(Error::Parse(
        "library response missing grid items in selected library tab".to_owned(),
    ))
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

fn section_grid_items<'a>(section: &'a Value) -> Result<Option<&'a [Value]>, Error> {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{library_grid_items, selected_library_tab};

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
}
