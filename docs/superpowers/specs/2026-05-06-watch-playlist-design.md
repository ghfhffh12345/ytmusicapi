# Watch Playlist Design

## Summary

This phase adds `get_watch_playlist` support to the Rust port of `sigma67/ytmusicapi` using the existing explicit continuation model already established in this repository.

The public API will:

- add a typed `WatchPlaylistQuery` request model
- add a dedicated `WatchTrack` response model
- return `Page<WatchTrack>` for the initial response
- add an explicit `get_watch_playlist_continuation(...)` method for caller-driven pagination

This phase intentionally does not expose the queue-level metadata that upstream Python returns on the first page, such as `playlistId`, `lyrics`, or `related`.

## Goals

- Support the upstream `get_watch_playlist` request controls:
  - `video_id`
  - `playlist_id`
  - `radio`
  - `shuffle`
- Keep continuation handling explicit and caller-driven.
- Preserve watch-specific track data instead of forcing the payload into existing search or library models.
- Match the crate's current public pagination style rather than the Python library's internal auto-pagination behavior.

## Non-Goals

- Auto-fetch continuations based on a caller-provided item limit.
- Expose first-page queue metadata such as generated playlist IDs, lyrics browse IDs, or related browse IDs.
- Generalize watch parsing into a shared cross-feature queue parser in this phase.
- Add write-action support for feedback tokens or listen-again token flows.

## Public API

Add a small typed request model:

```rust
pub struct WatchPlaylistQuery {
    pub video_id: Option<String>,
    pub playlist_id: Option<String>,
    pub radio: bool,
    pub shuffle: bool,
}
```

Add builder-style helpers following the current query-model style in the crate.

Validation rules:

- at least one of `video_id` or `playlist_id` must be present
- `shuffle` requires `playlist_id`
- `shuffle` must not be combined with `radio`

Allow `video_id` and `playlist_id` to be provided together. This matches upstream `ytmusicapi` v1.12.0 behavior and preserves the meaningful case of using a specific starting track within a playlist or album context.

Add two public client methods:

```rust
pub async fn get_watch_playlist(
    &self,
    query: WatchPlaylistQuery,
) -> Result<Page<WatchTrack>, Error>

pub async fn get_watch_playlist_continuation(
    &self,
    token: ContinuationToken,
) -> Result<Page<WatchTrack>, Error>
```

## Response Model

Add a dedicated watch-track model rather than reusing existing search or library song/video structs:

```rust
pub struct WatchTrack {
    pub video_id: String,
    pub title: String,
    pub duration: Option<String>,
    pub thumbnails: Vec<Thumbnail>,
    pub artists: Vec<ArtistRef>,
    pub album: Option<AlbumRef>,
    pub like_status: Option<LibraryLikeStatus>,
    pub video_type: Option<String>,
    pub year: Option<String>,
    pub views: Option<String>,
    pub is_in_library: Option<bool>,
    pub counterpart: Option<Box<WatchTrack>>,
}
```

Notes:

- `counterpart` is watch-specific and should be preserved when the upstream payload includes a song/video switch target.
- `like_status` should reuse the existing `LibraryLikeStatus` enum.
- Feedback-token and listen-again token fields should remain internal or unimplemented in this phase to keep the read-side model focused and stable.
- Queue-level metadata returned by upstream Python should not be exposed publicly in this phase.

## Request Construction

The initial request should target the upstream `next` endpoint and mirror v1.12.0 request semantics:

- include `videoId` when `video_id` is provided
- if `playlist_id` is absent and `video_id` is present, synthesize `playlistId = "RDAMVM{video_id}"`
- include validated `playlistId` when `playlist_id` is provided
- set the upstream `params` value for `radio` requests
- set the upstream `params` value for `shuffle` requests
- include the persistent playlist panel configuration only when neither `radio` nor `shuffle` is set

The continuation request should:

- post to the same `next` endpoint
- use the existing generic continuation request body builder if it is already compatible with the watch endpoint requirements
- accept only the server-issued `ContinuationToken`
- not require the caller to resubmit the original `WatchPlaylistQuery`

## Parsing Flow

Add a dedicated watch parsing module rather than merging this logic into `search` or `library`.

Initial response parsing should:

1. Navigate to the watch queue renderer and playlist panel.
2. Parse visible track rows into `WatchTrack`.
3. Preserve optional counterpart rows by recursively parsing the counterpart track payload into `WatchTrack`.
4. Extract an optional continuation token from the playlist panel.
5. Return `Page<WatchTrack>`.

Continuation parsing should:

1. Parse the `playlistPanelContinuation` payload returned by the `next` continuation response.
2. Parse continuation rows into `WatchTrack`.
3. Extract the next optional continuation token.
4. Return `Page<WatchTrack>`.

The parser may read queue-level metadata internally while locating the correct renderer subtree, but it must discard that metadata before building the public result.

## Transport and Authentication

`get_watch_playlist` should follow the same default transport posture as `search`:

- work anonymously by default
- avoid browser-auth gating unless implementation evidence shows the endpoint is unavailable for the required cases without browser auth

If authenticated transport later proves necessary for some watch subcases, that should be added as a focused follow-up rather than broadening this phase preemptively.

## Testing

Add coverage at three layers.

Query validation tests:

- reject missing `video_id` and `playlist_id`
- reject `shuffle` without `playlist_id`
- reject `radio` combined with `shuffle`
- accept `video_id` only
- accept `playlist_id` only
- accept both `video_id` and `playlist_id`

Fixture-backed parsing tests:

- parse a normal first-page watch response into non-empty `WatchTrack` items
- parse a radio first page if the payload shape differs materially
- parse a shuffled playlist first page if the payload shape differs materially
- parse a continuation response into non-empty `WatchTrack` items and a next continuation token when present
- verify counterpart parsing when the upstream payload includes a counterpart renderer

Transport tests:

- assert initial requests post to `next`
- assert continuation requests post a continuation body to `next`
- assert `RDAMVM{video_id}` synthesis only happens when `playlist_id` is absent
- assert `radio` and `shuffle` set the expected upstream request params

Ignored live coverage is optional and secondary to sanitized fixture coverage.

## Design Rationale

This design deliberately follows the public pagination style already established in this crate for `search` and library APIs:

- first page returns a `Page<T>`
- continuation is explicit and token-driven
- callers control whether to paginate further

That is intentionally different from upstream Python, which auto-fetches additional watch items internally to satisfy a `limit`. The Rust port should stay internally consistent instead of mixing implicit and explicit pagination styles across features.

## Scope Check

This is a single implementation phase:

- one request model
- one response model
- two public client methods
- one dedicated parser path
- tests and fixtures for first-page and continuation behavior

No further decomposition is required at the spec stage.
