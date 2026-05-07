# Method-Specific Continuation Tokens Design

## Summary

This phase replaces the current unified public `ContinuationToken` with method-specific continuation token newtypes.

The goal is to make continuation tokens compile-time specific to the client method that produced them. A token returned by `get_library_artists` should only be accepted by `get_library_artists_continuation`, a token returned by `search` should only be accepted by `search_continuation`, and so on.

This is a breaking public API cleanup. The crate is still early enough that establishing a precise pagination contract is more important than preserving the old unified token surface.

## Goals

- Remove the public unified `ContinuationToken`.
- Add one public continuation token type per continuation-enabled client method.
- Keep continuation tokens opaque and string-backed.
- Make token constructors infallible.
- Preserve the existing explicit caller-driven pagination model.
- Reuse shared continuation request construction internally.
- Keep serialized page output shape compatible by serializing typed tokens as plain strings.

## Non-Goals

- Runtime validation of token strings.
- Backward-compatible aliases or deprecated escape hatches for the old `ContinuationToken`.
- Auto-pagination helpers or iterator abstractions.
- New continuation-enabled endpoint families.
- Classification by every upstream JSON container shape.

## Public API

Replace the old exported `ContinuationToken` with these method-specific token types:

```rust
SearchContinuationToken
WatchPlaylistContinuationToken
LibraryPlaylistsContinuationToken
LibraryArtistsContinuationToken
LibraryAlbumsContinuationToken
LibrarySubscriptionsContinuationToken
LibraryChannelsContinuationToken
LibraryPodcastsContinuationToken
LibrarySongsContinuationToken
LikedSongsContinuationToken
SavedEpisodesContinuationToken
```

Each token is an opaque string-backed newtype:

```rust
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct LibraryArtistsContinuationToken(String);

impl LibraryArtistsContinuationToken {
    pub fn new(token: impl Into<String>) -> Self {
        Self(token.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

The same shape applies to every method-specific token type. The implementation may use a private macro or helper to avoid repeated boilerplate, but the public API should expose concrete token type names.

### Page Type

Change the shared page container from:

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub continuation: Option<ContinuationToken>,
}
```

to:

```rust
pub struct Page<T, C> {
    pub items: Vec<T>,
    pub continuation: Option<C>,
}
```

Simple list methods then return page types with their matching token:

```rust
pub async fn search(
    &self,
    query: SearchQuery,
) -> Result<Page<SearchResult, SearchContinuationToken>, Error>

pub async fn get_library_artists(
    &self,
) -> Result<Page<LibraryArtist, LibraryArtistsContinuationToken>, Error>

pub async fn get_watch_playlist(
    &self,
    query: WatchPlaylistQuery,
) -> Result<Page<WatchTrack, WatchPlaylistContinuationToken>, Error>
```

### Continuation Methods

Each continuation method accepts only the matching token type:

```rust
pub async fn search_continuation(
    &self,
    token: SearchContinuationToken,
) -> Result<Page<SearchResult, SearchContinuationToken>, Error>

pub async fn get_library_artists_continuation(
    &self,
    token: LibraryArtistsContinuationToken,
) -> Result<Page<LibraryArtist, LibraryArtistsContinuationToken>, Error>

pub async fn get_watch_playlist_continuation(
    &self,
    token: WatchPlaylistContinuationToken,
) -> Result<Page<WatchTrack, WatchPlaylistContinuationToken>, Error>
```

The same pattern applies to playlists, albums, subscriptions, channels, podcasts, songs, liked songs, and saved episodes.

### Wrapper Pages

Liked songs and saved episodes keep their dedicated wrapper page types, but their continuation fields become method-specific:

```rust
pub struct LikedSongsPage {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<LikedSongItem>,
    pub thumbnails: Vec<Thumbnail>,
    pub continuation: Option<LikedSongsContinuationToken>,
}

pub struct SavedEpisodesPage {
    pub playlist_id: String,
    pub title: String,
    pub items: Vec<SavedEpisodeItem>,
    pub thumbnails: Vec<Thumbnail>,
    pub continuation: Option<SavedEpisodesContinuationToken>,
}
```

## Data Flow

First-page parsers mint the exact token type for the method that produced the page:

- `parse_search_response` returns `Page<SearchResult, SearchContinuationToken>`.
- `parse_watch_playlist_response` returns `Page<WatchTrack, WatchPlaylistContinuationToken>`.
- `parse_library_artists_response` returns `Page<LibraryArtist, LibraryArtistsContinuationToken>`.
- Other library-like parsers follow the same method-specific pairing.

Continuation parsers return the same page/token pairing as their first-page method. For example, `parse_library_artists_continuation` returns `Page<LibraryArtist, LibraryArtistsContinuationToken>`.

Parser modules should not expose raw strings for continuation data. They should convert payload strings into the method-specific token at the parsing boundary.

## Request Construction

Continuation request construction should remain shared. The request body shape is still:

```json
{
  "continuation": "...",
  "context": {
    "client": {
      "clientName": "WEB_REMIX",
      "clientVersion": "..."
    }
  }
}
```

Use a crate-private trait or equivalent helper so `build_continuation_body` can accept any method-specific token without converting through public raw strings:

```rust
pub(crate) trait ContinuationTokenValue {
    fn as_str(&self) -> &str;
}
```

All public token types implement this private trait. The request builder can then accept `&impl ContinuationTokenValue`.

## Watch Continuation Shape

The watch parser should keep supporting both observed upstream continuation containers:

- `nextContinuationData`
- `nextRadioContinuationData`

Both containers should mint `WatchPlaylistContinuationToken`. The public classification is by continuation-enabled method, not by every upstream JSON container.

## Error Handling

Token constructors are infallible:

```rust
pub fn new(token: impl Into<String>) -> Self
```

They store the provided string as-is, including an empty string. The crate should no longer return `Error::InvalidInput` for empty continuation tokens during token construction.

Stale, empty, or otherwise unusable tokens should fail through the normal request path:

- transport failure
- non-success HTTP status
- JSON decode failure
- semantic parse failure

Existing input validation for query models remains unchanged.

## Serialization

All method-specific token types use transparent serialization. Public JSON output for pages still serializes continuation values as strings:

```json
{
  "items": [],
  "continuation": "next-page-token"
}
```

This keeps serialized output compatible even though Rust types become more precise.

## Compatibility

This is a breaking Rust API change:

- `ContinuationToken` is removed from public exports.
- `Page<T>` becomes `Page<T, C>`.
- Every continuation-enabled method signature changes.
- Existing callers must pass the token returned by the matching first-page or continuation page.

This break is intentional because the crate's public pagination contract is still being established.

## Testing

Update fixture-backed and transport tests to use the exact token type for each surface:

- search tests use `SearchContinuationToken`
- watch tests use `WatchPlaylistContinuationToken`
- library playlist tests use `LibraryPlaylistsContinuationToken`
- library artist tests use `LibraryArtistsContinuationToken`
- library album tests use `LibraryAlbumsContinuationToken`
- library subscription tests use `LibrarySubscriptionsContinuationToken`
- library channel tests use `LibraryChannelsContinuationToken`
- library podcast tests use `LibraryPodcastsContinuationToken`
- library song tests use `LibrarySongsContinuationToken`
- liked songs tests use `LikedSongsContinuationToken`
- saved episodes tests use `SavedEpisodesContinuationToken`

Add or update serialization coverage proving that typed tokens serialize as plain strings inside page output.

Remove the old empty-token rejection test or replace it with an infallible-constructor test proving `new("")` preserves the raw value.

Ignored live smoke tests should continue to drive continuations by passing the returned typed token directly into the matching continuation method.

## Scope Check

This is one implementation phase:

- replace the public token model
- update public page and wrapper page types
- update parser return types and token construction
- update continuation method signatures
- keep shared request construction through a private token-value abstraction
- update tests and examples

No additional endpoint families or pagination helpers are included.
