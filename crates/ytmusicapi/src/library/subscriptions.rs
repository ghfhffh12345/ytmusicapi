use serde_json::Value;

use crate::{Error, LibrarySubscription, Page};

use super::core::{
    continuation_shelf, continuation_shelf_contents, extract_continuation_token,
    library_shelf_contents, library_shelf_continuation, parse_artist_like_row,
};

pub(crate) fn parse_library_subscriptions_response(
    response: &Value,
) -> Result<Page<LibrarySubscription>, Error> {
    Ok(Page {
        items: library_shelf_contents(response)?
            .iter()
            .map(parse_library_subscription)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: library_shelf_continuation(response)?,
    })
}

pub(crate) fn parse_library_subscriptions_continuation(
    response: &Value,
) -> Result<Page<LibrarySubscription>, Error> {
    Ok(Page {
        items: continuation_shelf_contents(response)?
            .iter()
            .map(parse_library_subscription)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: extract_continuation_token(continuation_shelf(response)?)?,
    })
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
