use serde_json::Value;

use crate::{Error, LibraryChannel};

use super::{artists::parse_artist_like_row, core::library_shelf_contents};

pub(crate) fn parse_library_channels_response(
    response: &Value,
) -> Result<Vec<LibraryChannel>, Error> {
    library_shelf_contents(response)?
        .iter()
        .map(parse_library_channel)
        .collect()
}

fn parse_library_channel(item: &Value) -> Result<LibraryChannel, Error> {
    let row = parse_artist_like_row(item, "channel shelf item")?;

    Ok(LibraryChannel {
        browse_id: row.browse_id,
        name: row.name,
        subscribers: row.subscribers,
        thumbnails: row.thumbnails,
    })
}
