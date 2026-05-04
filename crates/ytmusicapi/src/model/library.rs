use serde::Serialize;

use crate::{AlbumRef, ArtistRef, Error, Thumbnail};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LibraryLikeStatus {
    Like,
    Indifferent,
    Dislike,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ContinuationToken(String);

impl ContinuationToken {
    pub fn new(token: impl Into<String>) -> Result<Self, Error> {
        let token = token.into();
        if token.trim().is_empty() {
            return Err(Error::InvalidInput(
                "continuation token must not be empty".to_owned(),
            ));
        }

        Ok(Self(token))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ContinuationToken>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub account_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_handle: Option<String>,
    pub account_photo_url: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAlbum {
    pub browse_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist_id: Option<String>,
    pub title: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryArtist {
    pub browse_id: String,
    pub artist: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySubscription {
    pub browse_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryChannel {
    pub browse_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPodcastChannel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPodcast {
    pub title: String,
    pub browse_id: String,
    pub podcast_id: String,
    pub channel: LibraryPodcastChannel,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryPlaylist {
    pub playlist_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_count: Option<u32>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySong {
    pub video_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<AlbumRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_status: Option<LibraryLikeStatus>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedSongItem {
    pub video_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<AlbumRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_status: Option<LibraryLikeStatus>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedSongs {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<LikedSongItem>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedSongsPage {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<LikedSongItem>,
    pub thumbnails: Vec<Thumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ContinuationToken>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedEpisodeItem {
    pub video_id: String,
    pub title: String,
    pub channel: String,
    pub podcast: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedEpisodes {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<SavedEpisodeItem>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedEpisodesPage {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<SavedEpisodeItem>,
    pub thumbnails: Vec<Thumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ContinuationToken>,
}
