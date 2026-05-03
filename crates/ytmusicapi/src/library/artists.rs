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
    let row = parse_artist_like_row(item, "artist shelf item")?;

    Ok(LibraryArtist {
        browse_id: row.browse_id,
        artist: row.name,
        subscribers: row.subscribers,
        thumbnails: row.thumbnails,
    })
}

pub(crate) struct ArtistLikeRow {
    pub(crate) browse_id: String,
    pub(crate) name: String,
    pub(crate) subscribers: Option<String>,
    pub(crate) thumbnails: Vec<crate::Thumbnail>,
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
