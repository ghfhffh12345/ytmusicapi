# ytmusicapi Rust Port: Foundation Search Design

Date: 2026-04-30
Upstream baseline: `sigma67/ytmusicapi` `v1.12.0` (released April 29, 2026)
Status: Approved design for first implementation slice

## Summary

The first implementation slice will establish an idiomatic Rust foundation for a `ytmusicapi` crate by shipping one usable, async-first vertical slice: unauthenticated public-catalog `search`.

This slice intentionally does not attempt broad parity with the Python project. Instead, it validates the core crate architecture, request flow, parsing strategy, and testing model against one real endpoint, while using `v1.12.0` as the behavioral baseline.

## Goals

- Provide a public `YtMusic` client with an async `search` API.
- Use idiomatic Rust types for requests, results, and errors.
- Return strongly typed search results from day one.
- Build on established crates instead of custom infrastructure.
- Create internal boundaries that can support later endpoints and authentication without a crate-wide rewrite.

## Non-Goals

- Authentication of any kind, including browser headers and OAuth.
- Authenticated scopes such as library or uploads.
- Playlist mutation, library mutation, uploads, podcasts, lyrics, or any other non-search endpoint.
- Complete upstream feature parity in the first slice.
- Live-network reliability guarantees in CI.

## Scope

This design covers only public-catalog search in the unauthenticated case.

Supported in slice one:

- Construction of a reusable async client.
- Public search requests using typed input models.
- Public search with the default mixed-result mode and explicit filter support for `songs`, `videos`, `albums`, `artists`, and `playlists`.
- Strongly typed top-level search result variants and typed shared substructures.
- Unit and fixture-based integration tests for request and parsing behavior.

Explicitly out of scope in slice one:

- `scope=library`
- `scope=uploads`
- Authenticated search behavior
- Account-specific responses
- Any endpoint other than `search`

## Public API Direction

The Rust crate will optimize for idiomatic Rust rather than Python method parity.

The initial public API should center on a single client type, tentatively:

```rust
pub struct YtMusic { ... }
```

with an async method conceptually shaped like:

```rust
pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, Error>;
```

The exact constructor details can be decided during planning, but the public surface should keep these properties:

- Async-first API only in this slice.
- No runtime-global state.
- Reusable client instance backed by a single shared HTTP client.
- Typed request and response models instead of `serde_json::Value` at the boundary.

## Architecture

The first slice should be implemented as a small set of stable internal layers:

- `client`: public `YtMusic` API and orchestration of endpoint calls.
- `search`: endpoint-specific request building and response parsing.
- `model`: domain models shared by public APIs, including search result types and supporting structs.
- `error`: crate-wide public error type and internal conversion helpers.

This is intentionally not a direct Rust translation of the Python project's mixin layout. The crate should preserve behavior where relevant, but organize internals around Rust cohesion and testability.

The design target is that future endpoints such as `get_song` or authenticated search can plug into these layers instead of forcing a reorganization.

## Dependencies

Use established, well-vetted crates for foundational concerns:

- `reqwest` for HTTP transport
- `serde` and `serde_json` for JSON serialization and deserialization
- `thiserror` for the public error type
- `tokio` for async tests and runtime support

Additional dependencies may be added during implementation only if they clearly simplify correctness or maintainability and are widely established in the Rust ecosystem.

## Request Model

The first slice should expose a typed search request model instead of raw strings for every parameter.

The request layer should model:

- query text
- optional public search filter limited to `songs`, `videos`, `albums`, `artists`, and `playlists`
- result limit
- `ignore_spelling` as an explicit boolean option

Unsupported upstream combinations must not be silently accepted. If a caller attempts to use an unsupported feature through the Rust API, the crate should fail explicitly with a validation or unsupported-feature error.

Because authenticated and alternate-scope search are out of scope, the first slice should not expose public API shapes that imply those paths are available.

## Result Model

Search results should be strongly typed from day one.

The top-level result should be an enum, tentatively `SearchResult`, with variants for supported result categories such as:

- song
- video
- album
- artist
- playlist

Shared repeated structures should be modeled once and reused, for example:

- artist references
- thumbnails
- album references where applicable
- duration or duration text where applicable

Fields that are truly optional in upstream responses should be represented as `Option<T>`. Fields required to preserve the invariant of a supported result variant should remain required in Rust.

The crate should not expose raw upstream dictionaries in this slice.

## Data Flow

The request and response flow should be:

1. The caller constructs a `YtMusic` client.
2. The caller submits a typed `SearchQuery`.
3. The client validates slice-supported inputs before making an HTTP request.
4. The client builds the YouTube Music search request for the public catalog.
5. The HTTP layer sends the request through `reqwest`.
6. The response body is decoded as JSON.
7. A dedicated search parser translates the JSON into typed `SearchResult` values.
8. The typed results are returned to the caller.

Validation failures must stop before network I/O. Parsing must be isolated from transport so parser behavior can be tested from fixtures.

## Error Handling

The crate should expose a `thiserror`-based public error enum with distinct variants for:

- invalid input
- unsupported feature in this slice
- HTTP transport failure
- non-success HTTP status
- JSON decoding failure
- semantic parse failure caused by unexpected upstream response structure

The parser should be strict about fields required for supported variants. If upstream changes break the structural assumptions for a supported result type, the crate should return a parse error instead of silently producing partial or misleading typed data.

The parser may be permissive where upstream data is known to be optional or inconsistent, but that permissiveness must be represented explicitly through optional fields in the typed models.

## Compatibility Policy

Compatibility in slice one is behavioral, not structural.

This crate does not need to mirror Python internals or dictionary layout. Instead, compatibility with upstream `v1.12.0` means:

- the Rust crate supports an equivalent unauthenticated public-catalog search flow within the documented subset
- the Rust models preserve the key result categories and fields exposed by upstream search responses for supported variants
- unsupported upstream features are explicitly documented and rejected clearly

Using `v1.12.0` as the baseline also means fixtures, request assumptions, and typed result coverage should be derived from that version rather than from older docs or ad hoc current behavior.

## Testing Strategy

The first slice should use two primary test layers.

Unit tests:

- request validation behavior
- filter validation behavior
- request construction helpers
- parser behavior for individual result shapes
- error classification and conversion

Fixture-based integration tests:

- end-to-end `search` response parsing from captured `v1.12.0` fixtures
- mixed-result default search cases
- filtered search cases for stable anonymous baselines
- failure cases for malformed or structurally incompatible fixtures

Live-network tests may be added later, but they should be optional and kept out of the default deterministic test path because YouTube Music is an unstable private API.

### Anonymous Fixture Baseline Note

During implementation, anonymous upstream `v1.12.0` search responses proved unstable for some filtered searches in the current environment. In particular, anonymous `songs` searches consistently returned empty results, and anonymous `videos` searches returned low-quality or podcast-like results that were not suitable as primary golden fixtures.

Because of that, fixture-driven parser acceptance in this slice is narrowed to the stable anonymous baselines:

- default mixed search
- albums
- artists
- playlists

The public Rust API still models `songs` and `videos` filters, but parser acceptance for those result types is not required from anonymous `v1.12.0` golden fixtures in this slice. If stable provenance is needed later, that work should use either a documented non-anonymous capture environment or a separately approved fixture strategy.

## Implementation Constraints

- Keep the crate async-first in this slice. Do not add a blocking client yet.
- Prefer established crates over custom solutions when a standard dependency fits the problem.
- Keep public types small and well-bounded so future authenticated or additional endpoint work can compose with them.
- Do not expose APIs that imply auth or alternate search scopes exist before they are implemented.
- Keep unsupported surface explicit in both documentation and runtime errors.

## Expected Outcome Of Slice One

After this slice lands, the crate should have:

- a reusable async client
- one working, typed public endpoint: `search`
- a stable error model
- a fixture-based parsing test harness aligned to upstream `v1.12.0`
- internal module boundaries suitable for expanding to additional endpoints

This should be enough to start the second design and planning cycle for either broader search support, authentication plumbing, or a second read-only endpoint such as `get_song`.
