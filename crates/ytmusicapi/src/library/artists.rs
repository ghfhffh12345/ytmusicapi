use serde_json::Value;

use crate::{Error, LibraryArtist, Page};

use super::core::{
    continuation_shelf, continuation_shelf_contents, extract_continuation_token,
    library_shelf_contents, library_shelf_continuation, parse_artist_like_row,
};

pub(crate) fn parse_library_artists_response(
    response: &Value,
) -> Result<Page<LibraryArtist>, Error> {
    Ok(Page {
        items: library_shelf_contents(response)?
            .iter()
            .map(parse_library_artist)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: library_shelf_continuation(response)?,
    })
}

pub(crate) fn parse_library_artists_continuation(
    response: &Value,
) -> Result<Page<LibraryArtist>, Error> {
    Ok(Page {
        items: continuation_shelf_contents(response)?
            .iter()
            .map(parse_library_artist)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: extract_continuation_token(continuation_shelf(response)?)?,
    })
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
