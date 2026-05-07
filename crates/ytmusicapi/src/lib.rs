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
    AccountInfo, LibraryAlbum, LibraryAlbumsContinuationToken, LibraryArtist,
    LibraryArtistsContinuationToken, LibraryChannel, LibraryChannelsContinuationToken,
    LibraryLikeStatus, LibraryPlaylist, LibraryPlaylistsContinuationToken, LibraryPodcast,
    LibraryPodcastChannel, LibraryPodcastsContinuationToken, LibrarySong,
    LibrarySongsContinuationToken, LibrarySubscription, LibrarySubscriptionsContinuationToken,
    LikedSongItem, LikedSongsContinuationToken, LikedSongsPage, Page, SavedEpisodeItem,
    SavedEpisodesContinuationToken, SavedEpisodesPage, SearchContinuationToken,
    WatchPlaylistContinuationToken,
};
pub use crate::model::search::{
    AlbumResult, ArtistResult, PlaylistResult, ProfileResult, SearchFilter, SearchQuery,
    SearchResult, SearchResultType, SongResult, VideoResult,
};
pub use crate::model::watch::{WatchPlaylistQuery, WatchTrack};
