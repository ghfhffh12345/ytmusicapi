# Library Continuation Support Design

## Summary

This phase adds caller-driven continuation support for the existing authenticated library reads in the Rust port of `sigma67/ytmusicapi`.

The phase covers:

- `get_library_playlists`
- `get_library_artists`
- `get_library_albums`
- `get_library_songs`
- `get_library_subscriptions`
- `get_library_channels`
- `get_library_podcasts`
- `get_liked_songs`
- `get_saved_episodes`

This phase does not cover search continuations. Search continuation support will be designed separately after the library continuation contract is proven.

## Goals

- Preserve the current authenticated browser-auth request model.
- Add explicit caller-driven continuation support rather than hidden auto-pagination.
- Use one shared public continuation token type.
- Keep the common paginated API shape simple for list-like endpoints.
- Preserve liked songs and saved episodes wrapper metadata where it is meaningful.

## Non-Goals

- Search continuation support
- Automatic fetching of all pages
- `limit` or `order` parameters
- Mutation or history APIs
- Continuation support for unauthenticated-only flows

## Public API

### Shared Pagination Types

Add a shared page container for the simple list families:

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub continuation: Option<ContinuationToken>,
}
```

Add one shared public continuation token newtype:

```rust
pub struct ContinuationToken(String);
```

The token type should be opaque and owned by the crate's public API. It should not expose endpoint-specific semantics.

### Updated First-Page Methods

Change the existing methods to return page-aware results:

- `get_library_playlists() -> Result<Page<LibraryPlaylist>, Error>`
- `get_library_artists() -> Result<Page<LibraryArtist>, Error>`
- `get_library_albums() -> Result<Page<LibraryAlbum>, Error>`
- `get_library_songs() -> Result<Page<LibrarySong>, Error>`
- `get_library_subscriptions() -> Result<Page<LibrarySubscription>, Error>`
- `get_library_channels() -> Result<Page<LibraryChannel>, Error>`
- `get_library_podcasts() -> Result<Page<LibraryPodcast>, Error>`

Replace the current wrapper return types for liked songs and saved episodes with page-aware wrapper types:

- `get_liked_songs() -> Result<LikedSongsPage, Error>`
- `get_saved_episodes() -> Result<SavedEpisodesPage, Error>`

### Wrapper Page Types

Add dedicated wrapper page types:

```rust
pub struct LikedSongsPage {
    pub playlist_id: String,
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub items: Vec<LikedSongItem>,
    pub continuation: Option<ContinuationToken>,
}

pub struct SavedEpisodesPage {
    pub playlist_id: String,
    pub title: String,
    pub thumbnails: Vec<Thumbnail>,
    pub items: Vec<SavedEpisodeItem>,
    pub continuation: Option<ContinuationToken>,
}
```

These wrapper page types exist because the first page carries real wrapper metadata that the simple `Page<T>` shape cannot represent without inventing meaningless fields for every endpoint.

### Continuation Methods

Add explicit continuation methods for every paginated library family:

- `get_library_playlists_continuation(token: ContinuationToken) -> Result<Page<LibraryPlaylist>, Error>`
- `get_library_artists_continuation(token: ContinuationToken) -> Result<Page<LibraryArtist>, Error>`
- `get_library_albums_continuation(token: ContinuationToken) -> Result<Page<LibraryAlbum>, Error>`
- `get_library_songs_continuation(token: ContinuationToken) -> Result<Page<LibrarySong>, Error>`
- `get_library_subscriptions_continuation(token: ContinuationToken) -> Result<Page<LibrarySubscription>, Error>`
- `get_library_channels_continuation(token: ContinuationToken) -> Result<Page<LibraryChannel>, Error>`
- `get_library_podcasts_continuation(token: ContinuationToken) -> Result<Page<LibraryPodcast>, Error>`
- `get_liked_songs_continuation(token: ContinuationToken) -> Result<LikedSongsPage, Error>`
- `get_saved_episodes_continuation(token: ContinuationToken) -> Result<SavedEpisodesPage, Error>`

The continuation methods should be explicit rather than overloading the base methods with optional parameters.

## Data Flow

### First Page

The first-page flow remains close to the current authenticated implementation:

1. Construct `YtMusic` from `browser.json`.
2. Build the existing authenticated browse request for the relevant library surface.
3. Parse the response items using the existing endpoint-specific parser split.
4. Extract an optional continuation token from the response.
5. Return the appropriate page type.

### Continuation Page

The continuation flow is separate and caller-driven:

1. Caller obtains a `ContinuationToken` from a first page or prior continuation page.
2. Caller passes the token into the relevant `*_continuation(...)` method.
3. The client builds a continuation request body for the relevant response family.
4. The response is parsed into the same public page type as the first page.
5. A new optional continuation token is returned when another page exists.

## Parser And Model Boundaries

The current internal split should remain intact:

- shared authenticated request and browse helpers
- shared library traversal helpers where the response structure is actually common
- endpoint-specific item parsers for playlists, artists, albums, songs, subscriptions, channels, podcasts, liked songs, and saved episodes

The continuation implementation may extract more shared traversal logic where clearly justified, but it should not force a fake generic abstraction over unrelated payload families.

### Simple List Families

For playlists, artists, albums, songs, subscriptions, channels, and podcasts, continuation pages should map directly into:

```rust
Page<T> {
    items,
    continuation,
}
```

No additional public page metadata is needed for these families.

### Liked Songs And Saved Episodes

Liked songs and saved episodes are different because the first page includes wrapper metadata:

- `playlist_id`
- `title`
- `thumbnails`

Continuation payloads may not repeat that metadata. The spec-defined behavior is:

- first-page methods populate all wrapper fields from the first-page payload
- continuation methods return the same wrapper page type
- if a continuation payload omits wrapper metadata, the crate should populate stable wrapper fields from the known wrapper identity for that surface rather than treating the omission as an ad hoc parser detail

In practice, that means the continuation parser for liked songs and saved episodes may need endpoint-defined default metadata derivation to preserve a stable return type.

## Error Handling

This phase stays within the existing `thiserror`-based `Error` model.

The implementation should continue distinguishing:

- missing or invalid browser-auth configuration
- transport failures
- non-success HTTP statuses
- JSON decode failures
- semantic parse failures for malformed first-page or continuation payloads

Continuation token misuse should only produce a dedicated validation-style error if the public token value is structurally invalid for the crate, such as an impossible empty token. Ordinary upstream rejection of a stale or invalid token should surface through the normal transport or response error path.

## Testing

The implementation should add coverage for:

- first-page extraction of continuation tokens for every supported paginated library family
- continuation request-body construction
- authenticated header behavior for continuation requests
- fixture-backed parsing of continuation payloads for:
  - at least one representative simple list family
  - liked songs
  - saved episodes
- regression coverage proving current first-page behavior still works after the return-type changes
- ignored live smoke coverage using the existing local browser-auth setup for representative continuation flows

Tests should continue to avoid committing sensitive browser-auth material. Only sanitized fixtures belong in the repository.

## Compatibility Notes

This phase intentionally changes the return types of the existing paginated library methods. That is acceptable at the crate's current stage because the goal is to establish a consistent caller-driven pagination contract before the API hardens further.

The compatibility target is behavioral parity with upstream `ytmusicapi` continuation behavior for the supported authenticated library methods, not reproduction of upstream Python internals.

## Deferred Work

The following remain out of scope for this phase:

- search continuations
- continuation support for history APIs
- automatic page accumulation helpers
- higher-level iterator abstractions
- mutation APIs that consume continuation-derived metadata
