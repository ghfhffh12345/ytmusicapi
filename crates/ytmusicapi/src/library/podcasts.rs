use serde_json::Value;

use crate::{Error, LibraryPodcast, LibraryPodcastChannel};

use super::core::{library_grid_items, optional_text, parse_thumbnails, required_text};

pub(crate) fn parse_library_podcasts_response(
    response: &Value,
) -> Result<Vec<LibraryPodcast>, Error> {
    library_grid_items(response)?
        .iter()
        .enumerate()
        .filter(|(index, item)| !(*index == 0 && is_leading_control_tile(item)))
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

fn is_leading_control_tile(item: &Value) -> bool {
    item.get("musicTwoRowItemRenderer")
        .is_some_and(|renderer| optional_podcast_browse_id(renderer).is_none())
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
