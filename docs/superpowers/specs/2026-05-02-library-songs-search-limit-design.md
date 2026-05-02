# Library Songs And Search Limit Removal Design

## Summary

The next slice should add authenticated `get_library_songs` support and remove the public search-result limit control from `SearchQuery`.

This keeps the library expansion moving on the highest-value remaining authenticated read while simplifying the search API around an explicit first-page-only contract. After this slice, `search` should return the full parsed first page from the active request path, and library reads should continue to expose only the default first page unless continuation support is designed explicitly later.

The behavioral baseline remains upstream `sigma67/ytmusicapi` `v1.12.0`, but the public Rust API should stay idiomatic and intentionally smaller than upstream where broader controls are not yet designed.

## Goals

- Add authenticated `get_library_songs` support.
- Return a dedicated `LibrarySong` model with stable core metadata only.
- Reuse and extend the shared authenticated library parser core where songs clearly need it.
- Remove `SearchQuery::limit` and `SearchQuery::with_limit(...)`.
- Make `search` always return the full first page of parsed results from the current transport path.
- Preserve the existing browser-authenticated and anonymous search transport behavior from the previous slice.

## Non-Goals

- Continuation or pagination support for library songs
- `limit` or `order` controls for library songs
- Mutation or action APIs for library songs
- Exposing feedback tokens, badges, set-video ids, or other richer action metadata publicly
- Any new public search pagination or shaping controls replacing the removed limit
- OAuth work or additional auth modes

## Scope

This slice contains two public-facing changes:

1. A new authenticated `get_library_songs` endpoint.
2. Removal of the search result limit option from the public query builder.

These belong in the same slice because they both reinforce the same contract: the library currently exposes first-page-only reads, and `search` should stop implying a caller-controlled paging or truncation surface before explicit continuation support exists.

## Public API

The library should add one new async method on `YtMusic`:

```rust
pub async fn get_library_songs(&self) -> Result<Vec<LibrarySong>, Error>;
```

This method should:

- require browser-authenticated client construction
- return only the default first page
- expose no `limit`, `order`, or continuation input

The search query API should be simplified:

- remove the `limit` field from `SearchQuery`
- remove `SearchQuery::with_limit(...)`
- remove validation related to zero or invalid limits
- remove any result truncation step from `YtMusic::search`

After this slice, `SearchQuery` should only model:

- search text
- optional `SearchFilter`
- spelling behavior

## Architecture

The `library` module should continue to be organized around:

- a shared authenticated library-browse core
- endpoint-specific parser modules

`get_library_songs` should be implemented as a new endpoint-specific parser module, such as `library::songs`, layered on top of the shared core used by playlists, artists, and albums.

The correct extraction boundary is:

- request construction, authenticated browse execution, tab selection, and container resolution remain in the shared core
- any clearly repeated item-walk helpers needed by songs should move into the shared core now
- song-specific renderer interpretation stays local to the songs parser

This is not the time to build a generic page or continuation abstraction. The shared core should grow only where repeated structure is already proven by the existing authenticated library endpoints plus songs.

## Data Model

This slice should add a dedicated `LibrarySong` top-level model rather than reusing the search song model directly.

`LibrarySong` should expose the stable core metadata:

- `video_id`
- `title`
- `artists`
- optional album reference
- optional duration
- `thumbnails`
- optional like status

Shared nested types should be reused where they already fit cleanly, especially thumbnail models and artist reference models. If there is not already a clean reusable album reference type, a small library-specific album reference is acceptable.

This slice should not expose:

- feedback tokens
- mutation endpoints
- like/dislike action payloads
- badges beyond what is needed to derive optional like status
- private parser internals needed only for future mutation work

## Data Flow

The `get_library_songs` flow should be:

1. The caller constructs `YtMusic` from a browser-auth file.
2. The caller invokes `get_library_songs`.
3. The client performs the authenticated library browse request.
4. The shared library core resolves the songs tab and extracts the item container.
5. The songs parser converts supported song rows into typed `LibrarySong` values.
6. The method returns the first page of parsed results.

The `search` flow should keep the current transport behavior from the previous slice:

- prefer browser-authenticated search when browser auth is configured
- fall back to anonymous search on request transport or HTTP status failures
- keep parse and JSON failures as hard errors rather than silently retrying through a different parser path

The only behavioral change to search in this slice is removal of caller-controlled truncation. Parsed first-page results should now be returned as-is.

## Parsing Rules

The songs parser should be strict about stable identity and title fields. If a row that is expected to represent a library song lacks required identity fields such as the song video id or title structure, that should be a semantic parse error rather than a silently skipped result.

Optional metadata should remain optional:

- album reference
- duration
- like status

This matches the reality that authenticated library song rows are not always uniform across accounts or response variants.

The parser should remain endpoint-specific. The shared library core may help walk item containers, but it should not absorb song semantics or become a generic typed model factory.

## Error Handling

This slice should continue using the existing `thiserror`-based crate error type. No new public error enum is needed.

The implementation should continue to distinguish:

- missing or invalid browser-auth configuration
- transport failures
- non-success HTTP statuses
- JSON decode failures
- semantic parse failures caused by upstream response drift

For `get_library_songs`, parse failures should be explicit when:

- the songs tab cannot be resolved
- the selected tab content cannot be traversed into song rows
- a required song identity or title field is malformed or missing

Removing `SearchQuery::limit` should not change the existing error model. It only removes one class of invalid input that will no longer exist.

## Testing

This slice should add three layers of coverage for `get_library_songs`:

- unit or fixture-backed parser tests for song-row extraction and field decoding
- transport/request tests for the authenticated library browse path
- an ignored live smoke test using the existing local browser-auth setup

It should also add regression coverage for the shared library core so that songs-related refactoring does not break:

- `get_library_playlists`
- `get_library_artists`
- `get_library_albums`

Search-related tests should be updated to reflect limit removal:

- remove tests that validate `with_limit(...)`
- remove tests that validate zero-limit rejection
- remove tests that expect truncation of parsed search results
- replace them with assertions that `search` returns the full parsed first page

Live tests should continue to remain ignored by default and rely on the local browser-auth setup already used in earlier slices.

## Compatibility

For this slice, compatibility with upstream `ytmusicapi` `v1.12.0` means:

- browser-authenticated clients can read the default first page of library songs
- the returned Rust model captures the stable, meaningful subset of upstream song metadata
- no unsupported surface such as `limit`, `order`, or continuation is exposed prematurely
- `search` still supports the same query/filter behavior as the previous slice, minus the Rust-only truncation control that is being intentionally removed

Compatibility is behavioral, not structural. The Rust crate does not need to reproduce Python mixins, dict shapes, or internal parser organization.

## Acceptance Criteria

This slice is complete when:

- `YtMusic::get_library_songs()` exists and returns typed `LibrarySong` results for authenticated clients
- the implementation reuses the shared library core without introducing a speculative continuation abstraction
- `SearchQuery` no longer exposes `limit` or `with_limit(...)`
- `YtMusic::search` no longer truncates parsed results
- test coverage exists for library songs parsing and transport
- regression coverage protects existing library endpoints after the shared-core extraction
- search tests reflect the new full-first-page behavior
- the ignored live smoke path covers `get_library_songs` with local browser auth

## Rationale

`get_library_songs` is the highest-value remaining authenticated library read, but it should not drag continuation or mutation design into the same slice. A focused first-page-only API keeps the public contract honest and useful.

Removing `SearchQuery::limit` simplifies the search surface in the same direction. The crate should not imply support for caller-driven result shaping when the underlying implementation is still intentionally first-page oriented. If continuation or explicit page objects are added later, they should be designed directly rather than approximated through client-side truncation knobs.
