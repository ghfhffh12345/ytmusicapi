use serde::Serialize;

use crate::Thumbnail;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSongResponse {
    pub video_details: SongVideoDetails,
    pub playability_status: SongPlayabilityStatus,
    pub streaming_data: SongStreamingData,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub microformat: Option<SongMicroformat>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongVideoDetails {
    pub video_id: String,
    pub title: String,
    pub length_seconds: u32,
    pub channel_id: String,
    pub author: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thumbnails: Vec<Thumbnail>,
    pub allow_ratings: bool,
    pub view_count: String,
    pub is_owner_viewing: bool,
    pub is_crawlable: bool,
    pub is_private: bool,
    pub is_unplugged_corpus: bool,
    pub is_live_content: bool,
    pub is_tvfilm_video: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub music_video_type: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongPlayabilityStatus {
    pub status: String,
    pub playable_in_embed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_params: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_only_availability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_mode: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongStreamingData {
    pub expires_in_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_abr_streaming_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub formats: Vec<SongStreamFormat>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adaptive_formats: Vec<SongStreamFormat>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongStreamFormat {
    pub itag: u32,
    pub mime_type: String,
    pub bitrate: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub average_bitrate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality_ordinal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub projection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_info: Option<SongColorInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_quality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_channels: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loudness_db: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_absolute_loudness_lkfs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approx_duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_replication: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xtags: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init_range: Option<SongByteRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_range: Option<SongByteRange>,
    pub signature_cipher: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongByteRange {
    pub start: String,
    pub end: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongColorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primaries: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_characteristics: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_coefficients: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SongMicroformat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_canonical: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publish_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view_count: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_countries: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noindex: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unlisted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_safe: Option<bool>,
}
