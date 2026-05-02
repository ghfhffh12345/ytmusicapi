use serde::Serialize;

use crate::{AlbumRef, ArtistRef, Thumbnail};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LibraryLikeStatus {
    Like,
    Indifferent,
    Dislike,
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
