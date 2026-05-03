use serde_json::Value;

use crate::{Error, LibraryPodcast, LibraryPodcastChannel, Page};

use super::core::{
    continuation_grid, continuation_grid_items, extract_continuation_token,
    library_grid_continuation, library_grid_items, optional_text, parse_thumbnails, required_text,
};

pub(crate) fn parse_library_podcasts_response(
    response: &Value,
) -> Result<Page<LibraryPodcast>, Error> {
    Ok(Page {
        items: parse_podcast_items(library_grid_items(response)?)?,
        continuation: library_grid_continuation(response)?,
    })
}

pub(crate) fn parse_library_podcasts_continuation(
    response: &Value,
) -> Result<Page<LibraryPodcast>, Error> {
    Ok(Page {
        items: parse_podcast_items(continuation_grid_items(response)?)?,
        continuation: extract_continuation_token(continuation_grid(response)?)?,
    })
}

fn parse_podcast_items(items: &[Value]) -> Result<Vec<LibraryPodcast>, Error> {
    items
        .iter()
        .enumerate()
        .filter(|(index, item)| !(*index == 0 && is_leading_add_podcasts_tile(item)))
        .map(|(_, item)| parse_library_podcast(item))
        .collect()
}

fn parse_library_podcast(item: &Value) -> Result<LibraryPodcast, Error> {
    let renderer = item.get("musicTwoRowItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicTwoRowItemRenderer in podcast grid item".to_owned(),
        )
    })?;
    let browse_id = podcast_browse_id(renderer)?;

    Ok(LibraryPodcast {
        title: required_text(renderer, "/title/runs/0/text")?,
        podcast_id: browse_id_to_podcast_id(&browse_id),
        browse_id,
        channel: LibraryPodcastChannel {
            id: optional_text(
                renderer,
                "/subtitle/runs/0/navigationEndpoint/browseEndpoint/browseId",
            ),
            name: required_text(renderer, "/subtitle/runs/0/text")?,
        },
        thumbnails: parse_thumbnails(renderer)?,
    })
}

fn is_leading_add_podcasts_tile(item: &Value) -> bool {
    item.get("musicTwoRowItemRenderer").is_some_and(|renderer| {
        optional_podcast_browse_id(renderer).is_none()
            && renderer.get("title").is_some()
            && renderer.get("subtitle").is_none()
            && renderer.get("thumbnailRenderer").is_none()
            && renderer.get("navigationEndpoint").is_none()
    })
}

fn podcast_browse_id(renderer: &Value) -> Result<String, Error> {
    optional_podcast_browse_id(renderer)
        .ok_or_else(|| Error::Parse("library response missing podcast browse id".to_owned()))
}

fn optional_podcast_browse_id(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/title/runs/0/navigationEndpoint/browseEndpoint/browseId",
    )
    .or_else(|| optional_text(renderer, "/navigationEndpoint/browseEndpoint/browseId"))
}

fn browse_id_to_podcast_id(browse_id: &str) -> String {
    browse_id
        .strip_prefix("VL")
        .or_else(|| browse_id.strip_prefix("MPSP"))
        .unwrap_or(browse_id)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_library_podcasts_response;
    use crate::Error;

    #[test]
    fn parse_library_podcasts_response_errors_on_unexpected_leading_tile() {
        let response = json!({
            "contents": {
                "singleColumnBrowseResultsRenderer": {
                    "tabs": [{
                        "tabRenderer": {
                            "selected": true,
                            "content": {
                                "sectionListRenderer": {
                                    "contents": [{
                                            "gridRenderer": {
                                                "items": [{
                                                    "musicTwoRowItemRenderer": {
                                                        "title": {
                                                            "runs": [{
                                                                "text": "Unexpected Tile"
                                                            }]
                                                        },
                                                        "subtitle": {
                                                            "runs": [{
                                                                "text": "Still not a podcast"
                                                            }]
                                                        }
                                                    }
                                                }, {
                                                    "musicTwoRowItemRenderer": {
                                                    "title": {
                                                        "runs": [{
                                                            "text": "Syntax",
                                                            "navigationEndpoint": {
                                                                "browseEndpoint": {
                                                                    "browseId": "MPSPpodcast123"
                                                                }
                                                            }
                                                        }]
                                                    },
                                                    "subtitle": {
                                                        "runs": [{
                                                            "text": "Syntax FM"
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

        let error = parse_library_podcasts_response(&response).unwrap_err();

        assert!(matches!(
            error,
            Error::Parse(message) if message == "library response missing podcast browse id"
        ));
    }
}
