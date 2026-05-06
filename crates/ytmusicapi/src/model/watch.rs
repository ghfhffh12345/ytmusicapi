use serde::Serialize;

use crate::{AlbumRef, ArtistRef, Error, LibraryLikeStatus, Thumbnail};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WatchPlaylistQuery {
    pub video_id: Option<String>,
    pub playlist_id: Option<String>,
    pub radio: bool,
    pub shuffle: bool,
}

impl WatchPlaylistQuery {
    pub fn new() -> Self {
        Self {
            video_id: None,
            playlist_id: None,
            radio: false,
            shuffle: false,
        }
    }

    pub fn with_video_id(mut self, video_id: impl Into<String>) -> Self {
        self.video_id = Some(video_id.into());
        self
    }

    pub fn with_playlist_id(mut self, playlist_id: impl Into<String>) -> Self {
        self.playlist_id = Some(playlist_id.into());
        self
    }

    pub fn radio(mut self) -> Self {
        self.radio = true;
        self
    }

    pub fn shuffle(mut self) -> Self {
        self.shuffle = true;
        self
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.video_id.as_deref().is_none_or(str::is_empty)
            && self.playlist_id.as_deref().is_none_or(str::is_empty)
        {
            return Err(Error::InvalidInput(
                "watch playlist query requires video_id or playlist_id".to_owned(),
            ));
        }

        if self.shuffle && self.playlist_id.as_deref().is_none_or(str::is_empty) {
            return Err(Error::InvalidInput(
                "watch playlist shuffle requires playlist_id".to_owned(),
            ));
        }

        if self.radio && self.shuffle {
            return Err(Error::InvalidInput(
                "watch playlist shuffle cannot be combined with radio".to_owned(),
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchTrack {
    pub video_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnails: Vec<Thumbnail>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<AlbumRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub like_status: Option<LibraryLikeStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_in_library: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub counterpart: Option<Box<WatchTrack>>,
}
