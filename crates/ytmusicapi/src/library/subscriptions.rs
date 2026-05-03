use serde_json::Value;

use crate::{Error, LibrarySubscription};

use super::core::{library_shelf_contents, parse_artist_like_row};

pub(crate) fn parse_library_subscriptions_response(
    response: &Value,
) -> Result<Vec<LibrarySubscription>, Error> {
    library_shelf_contents(response)?
        .iter()
        .map(parse_library_subscription)
        .collect()
}

fn parse_library_subscription(item: &Value) -> Result<LibrarySubscription, Error> {
    let row = parse_artist_like_row(item, "subscription shelf item")?;

    Ok(LibrarySubscription {
        browse_id: row.browse_id,
        name: row.name,
        subscribers: row.subscribers,
        thumbnails: row.thumbnails,
    })
}
