use serde::Serialize;

use crate::{ArtistRef, Thumbnail};

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
