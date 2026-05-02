use serde_json::Value;

use crate::{Error, LibrarySong};

pub(crate) fn parse_library_songs_response(_response: &Value) -> Result<Vec<LibrarySong>, Error> {
    Err(Error::Parse(
        "library songs parser not implemented".to_owned(),
    ))
}
