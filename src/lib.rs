mod client;
mod error;
mod model;
pub(crate) mod search;

pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::common::{AlbumRef, ArtistRef, Thumbnail};
pub use crate::model::search::{
    AlbumResult, ArtistResult, PlaylistResult, ProfileResult, SearchFilter, SearchQuery,
    SearchResult, SearchResultType, SongResult, VideoResult,
};
