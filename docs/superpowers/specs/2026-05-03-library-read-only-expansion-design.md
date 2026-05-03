# Library Read-Only Expansion Design

## Summary

The next slice should complete the remaining read-only Library methods from the upstream `ytmusicapi` Library section that fit the crate's current contract: authenticated calls, default behavior only, and no continuation, limit, or order surface.

This phase should add support for library subscriptions, channels, podcasts, liked songs, saved episodes, and account info. The behavioral baseline remains upstream `sigma67/ytmusicapi` `v1.12.0`, but the Rust API should stay intentionally smaller where broader controls are not yet designed.

## Goals

- Add authenticated `get_library_subscriptions` support.
- Add authenticated `get_library_channels` support.
- Add authenticated `get_library_podcasts` support.
- Add authenticated `get_liked_songs` support.
- Add authenticated `get_saved_episodes` support.
- Add authenticated `get_account_info` support.
- Keep the public API default-only for all six methods.
- Extend the existing authenticated library and playlist parsing layers instead of introducing parallel subsystems.

## Non-Goals

- Continuation or pagination support for any method in this slice
- `limit` or `order` inputs for any method in this slice
- History APIs
- Rating, subscribe, unsubscribe, or other mutation APIs
- OAuth work or additional auth modes
- Broader playlist or podcast endpoint expansion outside these Library methods

## Scope

This slice covers the remaining read-only methods from the upstream Library section that fit the current first-page/default-only contract:

1. `get_library_subscriptions`
2. `get_library_channels`
3. `get_library_podcasts`
4. `get_liked_songs`
5. `get_saved_episodes`
6. `get_account_info`

These belong together because they are all authenticated, side-effect-free reads, but they split naturally into four response families:

- artist-like library shelves
- podcast library shelves
- playlist-wrapper reads
- account identity data

## Public API

The library should add six new async methods on `YtMusic`:

```rust
pub async fn get_library_subscriptions(&self) -> Result<Vec<LibrarySubscription>, Error>;
pub async fn get_library_channels(&self) -> Result<Vec<LibraryChannel>, Error>;
pub async fn get_library_podcasts(&self) -> Result<Vec<LibraryPodcast>, Error>;
pub async fn get_liked_songs(&self) -> Result<LikedSongs, Error>;
pub async fn get_saved_episodes(&self) -> Result<SavedEpisodes, Error>;
pub async fn get_account_info(&self) -> Result<AccountInfo, Error>;
```

All six methods should:

- require browser-authenticated client construction
- expose no `limit`, `order`, or continuation input
- return only the default first page or default wrapper response shape

This slice should not introduce alternative constructors, page objects, or partial compatibility shims for upstream optional parameters.

## Architecture

The existing authenticated library architecture should remain the foundation:

- a shared authenticated request and browse core
- endpoint-specific parser modules layered on top

The new methods should fit into that structure as follows:

- `get_library_subscriptions` and `get_library_channels` should use the shared library browse core and separate endpoint parsers, with only small shared extraction helpers where the renderer shape is genuinely identical
- `get_library_podcasts` should use the shared library browse core but keep podcast-specific parsing isolated from artist/channel parsing
- `get_liked_songs` and `get_saved_episodes` should reuse the crate's playlist-style parsing path where practical, with thin method-specific wrappers around the result
- `get_account_info` should remain separate from shelf parsing because it represents account/profile data rather than a library tab listing

This is not the time to build a broad “all library reads” abstraction. Shared infrastructure should grow only where the current endpoints already prove structural repetition.

## Data Models

This slice should add dedicated top-level public models:

- `LibrarySubscription`
- `LibraryChannel`
- `LibraryPodcast`
- `LikedSongs`
- `SavedEpisodes`
- `AccountInfo`

Shared nested types should continue to be reused where they already fit, especially:

- thumbnail models
- artist- or channel-reference components where the meaning is stable
- playlist item models for liked songs and saved episodes if the existing playlist parser already produces a suitable typed shape

`LibrarySubscription` and `LibraryChannel` should remain distinct public types even if they share most fields. Their semantics differ, and the public API should preserve that difference rather than collapsing them into one generic artist-like model.

`LibraryPodcast` should expose the stable documented subset:

- title
- browse id
- podcast id
- channel name
- optional channel id
- thumbnails

The upstream “New Episodes” auto-playlist case should be representable as a normal `LibraryPodcast` entry with an optional channel id.

`LikedSongs` and `SavedEpisodes` should be dedicated wrapper types around playlist-style content. They should expose stable top-level metadata plus typed item collections, but this slice should not invent continuation or mutation fields that are not required for the current read path.

`AccountInfo` should be a small dedicated type with:

- account name
- channel handle
- account photo URL

## Data Flow

All six methods should follow the same high-level authenticated call flow:

1. The caller constructs `YtMusic` from a browser-auth file.
2. The caller invokes one of the new read-only methods.
3. The client sends the authenticated request through the existing browser-auth transport path.
4. Endpoint-specific parsing converts the response into typed Rust models.

The parsing path then diverges by response family:

- subscriptions, channels, and podcasts resolve the relevant library tab or shelf through the shared library core and parse items into dedicated models
- liked songs and saved episodes reuse the playlist-style parsing path where practical and wrap the result in dedicated top-level types
- account info parses a direct account/profile payload without forcing it through library-tab traversal

All methods in this slice return the default response only. No caller-controlled paging, ordering, or result shaping is supported.

## Parsing Rules

Parsers should remain strict about stable identity and required display fields for supported rows. If a row that is expected to represent a subscription, channel, podcast, or playlist item lacks required identity or title structure, that should be a semantic parse error rather than a silently skipped result.

Optional fields should remain optional where upstream behavior is known to vary, including:

- subscriber text
- optional channel id on podcast entries
- secondary metadata on playlist-style items where the shared parser already models it as optional

The shared core may extract repeated traversal logic, but endpoint-specific semantics must remain local to their parser modules. This slice should not turn the shared library layer into a generic typed renderer factory.

## Error Handling

This slice should continue using the existing `thiserror`-based crate error type. No new public error enum is needed.

The implementation should continue to distinguish:

- missing or invalid browser-auth configuration
- transport failures
- non-success HTTP statuses
- JSON decode failures
- semantic parse failures caused by upstream response drift

For this slice, parse failures should be explicit when:

- the expected library tab or shelf cannot be resolved
- the selected content cannot be traversed into supported items
- required identity or title fields are malformed or missing
- account info payloads lack the stable documented fields needed for `AccountInfo`

## Testing

This slice should add grouped coverage by response family:

- fixture-backed parser tests for subscriptions and channels
- fixture-backed parser tests for podcasts, including the optional-channel “New Episodes” shape
- fixture-backed or playlist-parser-backed tests for liked songs and saved episodes
- fixture-backed parser tests for account info

It should also add:

- transport or request-shape tests where request bodies differ materially between endpoint families
- regression coverage for any shared helpers extracted from the existing library or playlist parsers
- ignored live smoke coverage using the existing local browser-auth setup and `browser.txt`-derived auth flow

Live tests should remain ignored by default and should cover only the existence of usable results or stable top-level fields, not account-specific snapshots.

## Compatibility

For this slice, compatibility with upstream `ytmusicapi` `v1.12.0` means:

- authenticated clients can read the default subscriptions, channels, podcasts, liked songs, saved episodes, and account info views
- the returned Rust models capture the stable, meaningful subset of upstream data for each method
- unsupported upstream controls such as `limit`, `order`, and continuation are intentionally omitted
- the implementation reuses the current authenticated client and parser architecture instead of reproducing Python mixins or dict shapes directly

Compatibility is behavioral, not structural.

## Acceptance Criteria

This slice is complete when:

- all six new `YtMusic` methods exist and return typed results for authenticated clients
- dedicated top-level public models exist for each endpoint family
- the implementation extends the existing authenticated library and playlist parsing layers without introducing a speculative general framework
- no `limit`, `order`, or continuation inputs are exposed for these methods
- parser and transport coverage exists for each response family
- ignored live smoke coverage exists for the new methods using local browser auth

## Rationale

The upstream Library section still has several read-only methods left, but they are coherent as a single phase when limited to authenticated, side-effect-free, default-only behavior. Completing this cluster now gives the crate a substantially more useful authenticated library surface without mixing in mutations, history, or continuation design.

Keeping the public API intentionally smaller than upstream is still the right tradeoff. The crate should not expose `limit`, `order`, or pagination controls before continuation and multi-page behavior are designed explicitly.
