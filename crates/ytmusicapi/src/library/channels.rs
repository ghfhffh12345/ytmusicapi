use serde_json::Value;

use crate::{Error, LibraryChannel, LibraryChannelsContinuationToken, Page};

use super::core::{
    continuation_shelf, continuation_shelf_contents, extract_continuation_token,
    library_shelf_contents, library_shelf_continuation, parse_artist_like_row,
};

pub(crate) fn parse_library_channels_response(
    response: &Value,
) -> Result<Page<LibraryChannel, LibraryChannelsContinuationToken>, Error> {
    Ok(Page {
        items: library_shelf_contents(response)?
            .iter()
            .map(parse_library_channel)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: library_shelf_continuation(response, |token| {
            crate::LibraryChannelsContinuationToken::new(token)
        })?,
    })
}

pub(crate) fn parse_library_channels_continuation(
    response: &Value,
) -> Result<Page<LibraryChannel, LibraryChannelsContinuationToken>, Error> {
    Ok(Page {
        items: continuation_shelf_contents(response)?
            .iter()
            .map(parse_library_channel)
            .collect::<Result<Vec<_>, _>>()?,
        continuation: extract_continuation_token(continuation_shelf(response)?, |token| {
            crate::LibraryChannelsContinuationToken::new(token)
        }),
    })
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
