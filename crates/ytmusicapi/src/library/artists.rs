use serde_json::Value;

use crate::{Error, LibraryArtist};

use super::core::{
    library_shelf_contents, optional_runs_text, parse_thumbnails, required_runs_text, required_text,
};

pub(crate) fn parse_library_artists_response(
    response: &Value,
) -> Result<Vec<LibraryArtist>, Error> {
    library_shelf_contents(response)?
        .iter()
        .map(parse_library_artist)
        .collect()
}

fn parse_library_artist(item: &Value) -> Result<LibraryArtist, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicResponsiveListItemRenderer in artist shelf item"
                .to_owned(),
        )
    })?;

    Ok(LibraryArtist {
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        artist: required_runs_text(
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

fn first_token(value: String) -> Option<String> {
    value.split(' ').next().map(str::to_owned)
}
