use serde_json::Value;

use crate::{Error, LibraryArtist};

use super::core::{library_shelf_contents, parse_artist_like_row};

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
