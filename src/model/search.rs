use crate::{Error, search::params::encode_search_params};
use serde::Serialize;

use crate::model::common::{AlbumRef, ArtistRef, Thumbnail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SearchFilter {
    Songs,
    Videos,
    Albums,
    Artists,
    Playlists,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchResultType {
    Song,
    Video,
    Album,
    Artist,
    Profile,
    Playlist,
    Episode,
    Podcast,
}

impl SearchResultType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Song => "song",
            Self::Video => "video",
            Self::Album => "album",
            Self::Artist => "artist",
            Self::Profile => "profile",
            Self::Playlist => "playlist",
            Self::Episode => "episode",
            Self::Podcast => "podcast",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SearchResult {
    Song(SongResult),
    Video(VideoResult),
    Episode(VideoResult),
    Album(AlbumResult),
    Artist(ArtistResult),
    Profile(ProfileResult),
    Playlist(PlaylistResult),
    Podcast(PlaylistResult),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    pub video_id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<AlbumRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnails: Vec<Thumbnail>,
    pub is_explicit: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    pub title: String,
    pub video_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnails: Vec<Thumbnail>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub views: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub podcast: Option<AlbumRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    pub browse_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playlist_id: Option<String>,
    pub title: String,
    #[serde(rename = "type")]
    pub type_label: String,
    pub year: Option<String>,
    pub duration: Option<String>,
    pub is_explicit: bool,
    pub artists: Vec<ArtistRef>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artists: Vec<ArtistRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribers: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browse_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radio_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shuffle_id: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    pub browse_id: String,
    pub name: String,
    pub handle: String,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistResult {
    pub category: Option<String>,
    pub result_type: SearchResultType,
    pub browse_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_count: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchQuery {
    pub query: String,
    pub filter: Option<SearchFilter>,
    pub limit: usize,
    pub ignore_spelling: bool,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            filter: None,
            limit: 20,
            ignore_spelling: false,
        }
    }

    pub fn with_filter(mut self, filter: SearchFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn ignore_spelling(mut self) -> Self {
        self.ignore_spelling = true;
        self
    }

    pub fn validate(&self) -> Result<(), Error> {
        if self.query.trim().is_empty() {
            return Err(Error::InvalidInput("query must not be blank".to_owned()));
        }

        if matches!(
            self.filter,
            Some(SearchFilter::Songs | SearchFilter::Videos)
        ) {
            return Err(Error::UnsupportedFeature(
                "search parser currently supports only default mixed, albums, artists, and playlists responses"
                    .to_owned(),
            ));
        }

        if self.limit == 0 {
            return Err(Error::InvalidInput(
                "limit must be greater than zero".to_owned(),
            ));
        }

        Ok(())
    }

    pub fn encoded_params(&self) -> Option<String> {
        encode_search_params(self.filter, self.ignore_spelling)
    }
}
