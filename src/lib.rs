mod client;
mod error;
mod model;
pub(crate) mod search;

pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::common::{AlbumRef, ArtistRef, Thumbnail};
pub use crate::model::search::{
    AlbumResult, ArtistResult, PlaylistResult, SearchFilter, SearchQuery, SearchResult,
    SearchResultType, SongResult, VideoResult,
};

#[doc(hidden)]
pub mod internal {
    use serde_json::Value;

    use crate::{Error, SearchFilter, SearchResult, search::parse::parse_search_response};

    pub fn parse_search_response_for_test(
        response: &Value,
        filter: Option<SearchFilter>,
    ) -> Result<Vec<SearchResult>, Error> {
        parse_search_response(response, filter)
    }
}
