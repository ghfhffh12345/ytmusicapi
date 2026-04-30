mod client;
mod error;
mod model;
pub(crate) mod search;

pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::search::{SearchFilter, SearchQuery, SearchResult};
