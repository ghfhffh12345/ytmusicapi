# Search Continuation Support Design

## Summary

This phase adds caller-driven continuation support for `search` in the Rust port of `sigma67/ytmusicapi`.

The phase covers:

- unfiltered first-page search
- filtered first-page search
- continuation requests for unfiltered search
- continuation requests for filtered search

This phase changes the existing `search` return type from `Vec<SearchResult>` to `Page<SearchResult>` and adds an explicit `search_continuation(...)` method.

## Goals

- Reuse the shared pagination contract already established for library reads.
- Preserve the existing authenticated-vs-anonymous search transport selection.
- Support both filtered and unfiltered search continuations.
- Keep the public page shape minimal and avoid speculative search-only metadata.
- Make continuations explicit and caller-driven rather than hidden auto-pagination.

## Non-Goals

- Automatic fetching of all search pages
- Search-specific page metadata such as corrected-query state
- Continuation support for non-search endpoint families beyond what already exists
- New search filters or new search result variants
- API compatibility with the previous `Vec<SearchResult>` return type

## Public API

### Updated Search Method

Change the base search method to return the shared page container:

```rust
pub async fn search(&self, query: SearchQuery) -> Result<Page<SearchResult>, Error>
```

This change applies to both unfiltered and filtered search.

### Continuation Method

Add an explicit continuation method:

```rust
pub async fn search_continuation(
    &self,
    token: ContinuationToken,
) -> Result<Page<SearchResult>, Error>
```

The continuation method should accept only the server-issued token. It should not require callers to resubmit the original query or filter, because the continuation token is the canonical upstream cursor.

### Shared Page Contract

Reuse the existing shared pagination types:

```rust
pub struct Page<T> {
    pub items: Vec<T>,
    pub continuation: Option<ContinuationToken>,
}
```

```rust
pub struct ContinuationToken(String);
```

No dedicated `SearchPage` type should be introduced in this phase.

## Data Flow

### First Page

The first-page flow remains close to the current implementation:

1. Validate `SearchQuery`.
2. Bootstrap client config if needed to obtain the Innertube API key, client version, and anonymous fallback visitor id.
3. Build the first-page search request body.
4. Choose request headers based on how the client was constructed:
   - browser-auth headers when browser auth is configured
   - anonymous visitor headers otherwise
5. Parse typed search results.
6. Extract an optional continuation token.
7. Return `Page<SearchResult>`.

### Continuation Page

Continuation requests should follow the same transport model:

1. Caller obtains a `ContinuationToken` from a first page or prior continuation page.
2. Caller passes the token into `search_continuation(...)`.
3. Bootstrap client config if needed to obtain the current client version and request credentials.
4. Build the continuation request body from the token.
5. Choose browser-auth or anonymous headers using the same client-mode decision as first-page search.
6. Parse typed search results from the continuation payload.
7. Extract a new optional continuation token.
8. Return `Page<SearchResult>`.

## Parser And Model Boundaries

Search continuation parsing should remain in the search-specific parsing layer. The shared library continuation walkers are not a good abstraction for search because search responses use different containers and result renderers.

The internal split should be:

- shared search request builders for first-page and continuation bodies
- existing search result models and endpoint-specific parsing logic
- dedicated continuation parsing helpers where the response shape differs from first-page search

The implementation may extract small shared helpers if first-page and continuation search parsing genuinely overlap, but it should not force them through a brittle universal container walker.

### Search Metadata

This phase should not add search-specific public page metadata.

The crate does not currently model any stable search-only metadata such as:

- corrected query text
- suggestion text
- spelling recommendation state

Upstream `ytmusicapi` also documents search as a list-returning API rather than a rich page object with dedicated metadata fields. If later work proves that some search-specific metadata is stable and valuable, it can be added in a dedicated follow-up rather than guessed here.

## Error Handling

This phase stays within the existing `thiserror`-based `Error` model.

The implementation should continue distinguishing:

- invalid `SearchQuery` input
- invalid continuation token construction, such as an empty token
- transport failures
- non-success HTTP statuses
- JSON decode failures
- semantic parse failures for malformed first-page or continuation payloads

Continuation requests should not introduce a separate error family. Upstream rejection of a stale or invalid token should surface through the normal transport or response error path unless the crate can detect a structural token problem locally before the request is sent.

## Testing

The implementation should add coverage for:

- request-body construction for first-page and continuation search
- header selection for first-page and continuation search:
  - browser-auth headers when configured
  - anonymous visitor headers otherwise
- fixture-backed parsing of:
  - unfiltered first-page search with continuation extraction
  - unfiltered continuation search
  - filtered first-page search with continuation extraction
  - filtered continuation search
- regression coverage proving the existing typed search variants still parse correctly after the return-type change
- ignored live smoke coverage using the existing local browser-auth setup for representative unfiltered and filtered continuation flows when a continuation token is present

Tests should continue to avoid committing sensitive auth material. Only sanitized fixtures belong in the repository.

## Compatibility Notes

This phase intentionally changes the public `search` return type from `Vec<SearchResult>` to `Page<SearchResult>`. That is acceptable at the crate's current stage because the library surfaces already established the page-and-token contract for caller-driven pagination.

The compatibility target is behavioral parity with upstream `ytmusicapi` search continuation behavior for the supported first-page and continuation flows, not reproduction of upstream Python internals.

## Deferred Work

The following remain out of scope for this phase:

- automatic accumulation of all search pages
- richer search page metadata
- higher-level iterator abstractions over search pages
- changing the set of supported search filters
- any new browse, history, or mutation endpoint families
