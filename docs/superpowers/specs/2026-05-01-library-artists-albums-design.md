# Library Artists And Albums Design

Date: 2026-05-01
Upstream baseline: `sigma67/ytmusicapi` `v1.12.0`
Status: Approved design for next implementation slice

## Summary

The next slice expands the authenticated library-read surface by adding `get_library_artists` and `get_library_albums`.

This work should not be implemented as two isolated endpoint parsers. Instead, it should extract a shared internal library-browse core from the current `get_library_playlists` path, then layer endpoint-specific parsing for artists and albums on top of that shared traversal. Publicly, the slice remains narrow: browser-authenticated clients only, default upstream ordering only, and first-page behavior only.

## Goals

- Add authenticated `get_library_artists` support.
- Add authenticated `get_library_albums` support.
- Generalize the current library-tab traversal into shared infrastructure used by playlists, artists, and albums.
- Keep the public Rust API idiomatic and strongly typed.
- Preserve behavioral compatibility with upstream `ytmusicapi` `v1.12.0` for the documented default library reads.

## Non-Goals

- `get_library_songs`
- `get_library_subscriptions`
- Upload-library endpoints
- Pagination or continuation support
- `limit` or `order` parameters in this slice
- OAuth support
- Library mutation or playlist mutation

## Scope

This slice includes three tightly related changes:

1. Shared internal refactoring of the authenticated library-browse parser.
2. A new authenticated `get_library_artists` endpoint.
3. A new authenticated `get_library_albums` endpoint.

Anything outside those boundaries is explicitly deferred.

## Public API Direction

The library should continue to center its public surface on `YtMusic`. This slice adds two new async methods:

```rust
pub async fn get_library_artists(&self) -> Result<Vec<LibraryArtist>, Error>;
pub async fn get_library_albums(&self) -> Result<Vec<LibraryAlbum>, Error>;
```

The existing browser-authenticated construction flow remains the only supported auth mode for these endpoints.

These methods intentionally do not accept parameters in this slice. They should represent:

- default upstream library ordering
- authenticated first-page behavior only
- no explicit continuation token support

The public API should not imply that `limit`, `order`, or pagination already exist.

## Library Architecture

The `library` module should be restructured around a shared authenticated library-browse core plus endpoint-specific parsers.

The shared core should own:

- selection of the correct library tab from the authenticated browse response
- fallback handling for known upstream tab/container variants already observed in the playlist implementation
- extraction of the item list from the tab content
- shared structural validation and parse-error construction for tab/item discovery failures

Endpoint-specific parsing should remain separate:

- playlist item parsing stays focused on `LibraryPlaylist`
- artist item parsing produces `LibraryArtist`
- album item parsing produces `LibraryAlbum`

This boundary is important. Shared traversal logic should move into one place now because the repeated tab-selection and item-walk behavior is already real. But the final interpretation of each item renderer should remain endpoint-specific so future `get_library_songs` work does not force awkward generalization of result shapes.

## Result Models

This slice should add dedicated library result types rather than reusing search models directly.

### `LibraryArtist`

`LibraryArtist` should capture the stable subset of upstream authenticated library-artist data:

- artist browse identifier
- display name
- subscriber text when present
- thumbnails

### `LibraryAlbum`

`LibraryAlbum` should capture the stable subset of upstream authenticated library-album data:

- album browse identifier
- playlist identifier when present
- title
- album type label when present
- artist references
- year when present
- thumbnails

### Shared Nested Types

Existing shared nested models should be reused where they already match semantically, especially:

- thumbnail structures
- artist reference structures

Only the top-level library entities need to be new in this slice.

## Data Flow

Authenticated library reads should follow this flow:

1. The caller constructs `YtMusic` from a browser-auth file.
2. The caller invokes `get_library_artists` or `get_library_albums`.
3. The client performs the authenticated `browse` request through the existing browser-auth transport path.
4. The shared library core resolves the correct library tab and extracts the item container.
5. The endpoint-specific parser converts extracted renderers into typed `LibraryArtist` or `LibraryAlbum` values.
6. The typed results are returned to the caller.

This slice continues the current default-first approach: if upstream supports alternate ordering or continuations, those are intentionally left out of the public flow for now.

## Error Handling

The crate should continue using the existing `thiserror`-based public error model. This slice does not need a new error architecture, but it does need to preserve precise failure classification.

Relevant failure classes remain:

- browser-auth configuration or validation failure
- HTTP transport failure
- non-success HTTP status
- semantic parse failure caused by upstream structural drift

For this slice, semantic parse failures should remain explicit when:

- the expected library tab cannot be found
- the selected tab structure no longer matches supported container shapes
- an artist or album item that should be parseable is structurally malformed

The implementation should not hide these failures by returning an empty vector on structural mismatch.

## Compatibility Policy

Compatibility remains behavioral, not structural.

For upstream `ytmusicapi` `v1.12.0`, this slice should be considered compatible when:

- browser-authenticated clients can perform the default library artists read
- browser-authenticated clients can perform the default library albums read
- the returned typed models preserve the key stable fields of upstream results within the documented subset
- known unsupported features such as `limit`, `order`, and pagination are clearly absent rather than partially implemented

The Rust port does not need to mimic upstream Python mixin boundaries or dictionary shapes.

## Testing Strategy

This slice should stay deterministic by default and continue to keep live authenticated traffic out of normal CI.

Tests should cover three levels:

### Shared Library-Core Tests

- tab selection across the known supported library response variants
- item-container extraction for the supported tab content shapes
- structural failure behavior when the expected tab or container is missing

### Endpoint Parsing Tests

- successful typed parsing for artist items
- successful typed parsing for album items
- malformed item failures for both endpoints
- regression coverage ensuring `get_library_playlists` still works through the shared core after refactoring

### Local Live Smoke Coverage

Add ignored live smoke coverage, parallel to the current authenticated live test path, so local manual verification can exercise:

- `get_library_artists`
- `get_library_albums`

The repository-root browser-auth setup already used for local authenticated testing should remain local-only and must not become a committed credential fixture.

## Implementation Constraints

- Keep the crate async-first.
- Reuse the current authenticated `browse` transport path instead of inventing a separate request flow.
- Move repeated library-tab traversal into shared infrastructure now.
- Keep top-level result models endpoint-specific.
- Do not expose incomplete control surfaces such as `limit`, `order`, or continuation tokens before they are intentionally designed.

## Expected Outcome

After this slice lands, the crate should provide:

- a shared internal library-browse parser core used by authenticated library endpoints
- a typed `get_library_artists` API
- a typed `get_library_albums` API
- continued typed `get_library_playlists` behavior through the refactored shared core

That outcome expands the authenticated library foundation in a way that reduces duplication now and makes the future `get_library_songs` slice a simpler additive step instead of another parser fork.
