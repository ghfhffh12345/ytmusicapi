# Get Song Design

## Summary

This phase adds a new `get_song` method to the Rust port of `ytmusicapi` using the upstream `player` endpoint and a typed response model derived from real payloads.

The public API should expose a stable, practically useful subset of the `player` response without turning the crate into a raw player-payload mirror. The response model in this phase is based on the observed shapes in:

- `tmp/response1.json`
- `tmp/response2.json`
- `tmp/response3.json`

These payloads show a highly consistent top-level structure:

- `videoDetails`
- `playabilityStatus`
- `streamingData`
- `microformat.microformatDataRenderer`

They also show that stream rows expose `signatureCipher` as a raw string and do not include `player_url` or direct `url` fields in the observed samples.

## Goals

- Add a public `get_song(video_id, signature_timestamp)` client method.
- Model the useful read-side parts of the `player` payload with typed Rust structs.
- Preserve separate `formats` and `adaptive_formats` lists.
- Expose `signature_cipher` as a raw `String`.
- Keep the response grounded in real observed payloads rather than external integration assumptions.
- Validate `video_id` early with a user-facing error.

## Non-Goals

- Auto-discovering the signature timestamp.
- Parsing `signatureCipher` into structured fields.
- Exposing `player_url`.
- Exposing a raw untyped `serde_json::Value` player payload.
- Exposing `playbackTracking`, `heartbeatParams`, `playerConfig`, `storyboards`, `responseContext`, or `trackingParams` in this phase.
- Adding decipher helpers or URL-resolution helpers.

## Public API

Add a new low-level client method:

```rust
pub async fn get_song(
    &self,
    video_id: impl Into<String>,
    signature_timestamp: u32,
) -> Result<GetSongResponse, Error>
```

Validation rules:

- `video_id` must not be blank after trimming.
- `signature_timestamp` is required and passed through as caller-provided input.

The method should post to the upstream `player` endpoint.

## Response Model

The top-level response should expose the four useful public sections observed in the sample payloads:

```rust
pub struct GetSongResponse {
    pub video_details: SongVideoDetails,
    pub playability_status: SongPlayabilityStatus,
    pub streaming_data: SongStreamingData,
    pub microformat: Option<SongMicroformat>,
}
```

### Video Details

```rust
pub struct SongVideoDetails {
    pub video_id: String,
    pub title: String,
    pub length_seconds: u32,
    pub channel_id: String,
    pub author: String,
    pub thumbnails: Vec<Thumbnail>,
    pub allow_ratings: bool,
    pub view_count: String,
    pub is_owner_viewing: bool,
    pub is_crawlable: bool,
    pub is_private: bool,
    pub is_unplugged_corpus: bool,
    pub is_live_content: bool,
    pub is_tvfilm_video: bool,
    pub music_video_type: Option<String>,
}
```

Notes:

- `thumbnail.thumbnails` should reuse the existing shared `Thumbnail` model.
- `view_count` should remain a string because that is how it appears in the observed payloads.
- `music_video_type` should remain a string-backed field instead of introducing an enum in this phase.

### Playability Status

```rust
pub struct SongPlayabilityStatus {
    pub status: String,
    pub playable_in_embed: bool,
    pub context_params: Option<String>,
    pub audio_only_availability: Option<String>,
    pub playback_mode: Option<String>,
}
```

Mapping:

- `status` comes from `playabilityStatus.status`
- `playable_in_embed` comes from `playabilityStatus.playableInEmbed`
- `context_params` comes from `playabilityStatus.contextParams`
- `audio_only_availability` comes from `playabilityStatus.audioOnlyPlayability.audioOnlyPlayabilityRenderer.audioOnlyAvailability`
- `playback_mode` comes from `playabilityStatus.miniplayer.miniplayerRenderer.playbackMode`

The nested tracking parameter inside `audioOnlyPlayabilityRenderer` should remain internal in this phase.

### Streaming Data

```rust
pub struct SongStreamingData {
    pub expires_in_seconds: u64,
    pub server_abr_streaming_url: Option<String>,
    pub formats: Vec<SongStreamFormat>,
    pub adaptive_formats: Vec<SongStreamFormat>,
}
```

`server_abr_streaming_url` is present in all three observed payloads, but should still be optional in the public model because it is transport-like data and may be omitted in other payload variants.

### Stream Format

Use a single shared struct for both `formats` and `adaptiveFormats`:

```rust
pub struct SongStreamFormat {
    pub itag: u32,
    pub mime_type: String,
    pub bitrate: u64,
    pub average_bitrate: Option<u64>,
    pub content_length: Option<u64>,
    pub last_modified: Option<String>,
    pub quality: Option<String>,
    pub quality_label: Option<String>,
    pub quality_ordinal: Option<String>,
    pub projection_type: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fps: Option<u32>,
    pub color_info: Option<SongColorInfo>,
    pub audio_quality: Option<String>,
    pub audio_sample_rate: Option<u32>,
    pub audio_channels: Option<u32>,
    pub loudness_db: Option<f64>,
    pub track_absolute_loudness_lkfs: Option<f64>,
    pub approx_duration_ms: Option<u64>,
    pub high_replication: Option<bool>,
    pub xtags: Option<String>,
    pub init_range: Option<SongByteRange>,
    pub index_range: Option<SongByteRange>,
    pub signature_cipher: String,
}
```

```rust
pub struct SongByteRange {
    pub start: String,
    pub end: String,
}
```

```rust
pub struct SongColorInfo {
    pub primaries: Option<String>,
    pub transfer_characteristics: Option<String>,
    pub matrix_coefficients: Option<String>,
}
```

Notes:

- `signature_cipher` is required because it appears on every observed stream row.
- `signature_cipher` remains a raw `String`.
- No public `url` field should be modeled in this phase because none of the observed stream rows include one.
- `formats` rows and `adaptiveFormats` rows share one shape with optional fields, which better matches the observed payloads than splitting into separate public enums.
- `init_range` and `index_range` are optional because they appear on observed `adaptiveFormats` rows but not on observed `formats` rows.
- `xtags` appears only on one observed `formats` payload and must therefore remain optional.

### Microformat

Expose a focused subset of `microformat.microformatDataRenderer`:

```rust
pub struct SongMicroformat {
    pub url_canonical: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub publish_date: Option<String>,
    pub upload_date: Option<String>,
    pub view_count: Option<String>,
    pub available_countries: Vec<String>,
    pub tags: Vec<String>,
    pub noindex: Option<bool>,
    pub unlisted: Option<bool>,
    pub family_safe: Option<bool>,
}
```

Do not expose the entire nested renderer tree in this phase. The observed payloads contain additional nested fields such as `pageOwnerDetails`, renderer-local `videoDetails`, thumbnails, app-link metadata, and social metadata, but those are lower-value and more volatile than the focused subset above.

## Request Construction

The request should post to the upstream `player` endpoint using the existing JSON transport pattern already used elsewhere in the crate.

The request body should include:

- `videoId`
- `playbackContext.contentPlaybackContext.signatureTimestamp`
- the standard web remix client context used by the crate

This phase should not add:

- player JavaScript discovery
- signature timestamp discovery
- browser-auth-specific behavioral branching unless implementation evidence proves it is required

## Parsing Rules

Parse the response into the public model using these rules:

- `videoDetails`, `playabilityStatus`, and `streamingData` are required top-level sections.
- `microformat` is optional and should be parsed from `microformat.microformatDataRenderer`.
- Missing required top-level sections should return `Error::Parse(...)`.
- Missing required fields inside a stream item should fail parsing for the whole response rather than silently dropping rows.
- Optional nested sections should deserialize to `None` or empty vectors as appropriate.

Numeric conversion rules:

- Parse obviously numeric string fields into integers when their observed representation is consistently numeric:
  - `length_seconds`
  - `expires_in_seconds`
  - `bitrate`
  - `average_bitrate`
  - `content_length`
  - `approx_duration_ms`
  - `audio_sample_rate`
  - `width`
  - `height`
  - `fps`
  - `audio_channels`
- Keep server enum-like and label-like values as strings:
  - `music_video_type`
  - `audio_quality`
  - `quality`
  - `quality_label`
  - `quality_ordinal`
  - `projection_type`
  - `status`
  - `audio_only_availability`
  - `playback_mode`
- Keep `view_count` string-backed in both `SongVideoDetails` and `SongMicroformat`.

## Testing

Add fixture-backed tests using the observed sample payloads:

- `tmp/response1.json`
- `tmp/response2.json`
- `tmp/response3.json`

Coverage should include:

- successful parsing of all three payloads into `GetSongResponse`
- preservation of separate `formats` and `adaptive_formats`
- preservation of raw `signature_cipher`
- optional-field coverage for:
  - `xtags`
  - `color_info`
  - `loudness_db`
  - `track_absolute_loudness_lkfs`
  - `server_abr_streaming_url`
- expected differences in adaptive format counts across fixtures
- blank `video_id` validation
- transport coverage proving `get_song` posts to `player` and forwards the provided signature timestamp

Ignored live tests are optional and secondary to fixture-backed transport and parser coverage.

## Design Rationale

This design intentionally stays narrower than the full upstream `player` payload while being broader than a minimal metadata-only wrapper.

The public response includes the sections that were both:

- consistent across the observed payloads
- practically useful to downstream callers

It intentionally leaves out payload areas that are either internal, volatile, or not clearly appropriate for the stable Rust API surface in this phase.

The design also avoids shaping the API around any external deciphering library. The response model should reflect the real `player` payload first, and only then expose the stable subset that fits this crate's typed API conventions.

## Scope Check

This is a single implementation phase:

- one new client method
- one dedicated request builder path for `player`
- one dedicated parser path
- one new public response model family
- transport and fixture-backed tests

No further decomposition is required at the spec stage.
