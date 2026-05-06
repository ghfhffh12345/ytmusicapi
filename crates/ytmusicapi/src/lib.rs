mod auth;
mod client;
mod error;
pub(crate) mod library;
mod model;
pub(crate) mod search;
pub(crate) mod watch;

pub use crate::auth::setup_browser_auth;
pub use crate::client::{YtMusic, YtMusicBuilder};
pub use crate::error::Error;
pub use crate::model::common::{AlbumRef, ArtistRef, Thumbnail};
pub use crate::model::library::{
    AccountInfo, ContinuationToken, LibraryAlbum, LibraryArtist, LibraryChannel, LibraryLikeStatus,
    LibraryPlaylist, LibraryPodcast, LibraryPodcastChannel, LibrarySong, LibrarySubscription,
    LikedSongItem, LikedSongsPage, Page, SavedEpisodeItem, SavedEpisodesPage,
};
pub use crate::model::search::{
    AlbumResult, ArtistResult, PlaylistResult, ProfileResult, SearchFilter, SearchQuery,
    SearchResult, SearchResultType, SongResult, VideoResult,
};
pub use crate::model::watch::{WatchPlaylistQuery, WatchTrack};
