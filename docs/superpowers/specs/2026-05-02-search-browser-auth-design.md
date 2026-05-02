# Search Browser-Auth Design

Date: 2026-05-02
Upstream baseline: `sigma67/ytmusicapi` `v1.12.0`
Status: Approved design for next implementation slice

## Summary

This slice repairs filtered `search` behavior when the client has browser authentication configured.

The current implementation always sends search requests through the anonymous visitor path. In the current environment, that causes filtered `songs` searches to return a no-results payload and filtered `videos` searches to return a materially smaller shelf than the authenticated browser session. The fix is to keep the public `search` API unchanged, but make its transport prefer browser-auth headers when `YtMusic` was constructed from `browser.json`, while preserving the current anonymous fallback for unauthenticated clients.

## Goals

- Fix filtered `songs` searches for browser-authenticated clients.
- Improve filtered `videos` searches for browser-authenticated clients.
- Keep `YtMusic::search` as the single public catalog-search entrypoint.
- Reuse the existing browser-auth validation and header-construction path.
- Add deterministic regression coverage using sanitized authenticated fixtures.

## Non-Goals

- New public authenticated-search methods
- Search `scope` support such as `library` or `uploads`
- Continuation support
- Search result model redesign
- OAuth support
- Broader search-parity work beyond the filtered authenticated cases needed for this fix

## Public API Direction

The public API should remain unchanged in this slice:

```rust
pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, Error>;
```

The behavioral contract changes slightly:

- when `YtMusic` was built from `browser.json`, `search` should use that authenticated browser session
- when no browser auth is configured, `search` should continue using the existing anonymous visitor flow

No new method, parameter, or feature flag is needed to express this behavior. The client configuration already determines whether authenticated headers are available.

## Architecture

The internal `search` flow should keep its current high-level shape:

1. Validate the `SearchQuery`.
2. Bootstrap shared request config from the YouTube Music homepage.
3. Build the search request body from the typed query and bootstrap client version.
4. Send the request.
5. Decode and parse the JSON response into typed `SearchResult` values.

The main architectural change is step 4.

Bootstrap config should remain the source of:

- `INNERTUBE_API_KEY`
- `INNERTUBE_CONTEXT_CLIENT_VERSION`
- fallback visitor id for anonymous or mixed header construction

Header selection should become conditional:

- if `browser_auth` is present, build the request with the validated browser-auth header map, using the bootstrapped visitor id only as a fallback if the auth file lacks `x-goog-visitor-id`
- otherwise, use the existing anonymous header set with `content-type`, `user-agent`, and `x-goog-visitor-id`

This should match the pattern already used by authenticated `browse` requests so that `search` and `browse` do not drift into two incompatible auth implementations.

## Parsing And Fixture Scope

The parser should continue returning the existing typed search models and should not be redesigned in this slice.

The fixture expansion should focus only on the response shapes that are currently broken or materially degraded in anonymous mode:

- filtered `songs`
- filtered `videos`

Existing coverage for default mixed search and the other filtered categories should remain in place.

Sanitized authenticated fixtures should be captured from the current local browser-auth setup and committed only after removing sensitive data. These fixtures exist to validate that the parser handles real authenticated filtered shelves, not to preserve raw account state.

This slice should not claim that anonymous and authenticated filtered search are equivalent. Anonymous clients keep the current behavior and its current upstream limitations.

## Error Handling

The existing `thiserror`-based crate error surface should remain unchanged.

Relevant error classes remain:

- browser-auth file read, decode, or validation failure
- HTTP transport failure
- non-success HTTP status
- JSON decode failure
- semantic parse failure caused by incompatible upstream response structure

Authenticated `search` should reuse the same browser-auth header-construction path already used by authenticated `browse`, so malformed auth configuration fails before request dispatch rather than producing unclear downstream parsing errors.

## Testing Strategy

Testing should cover three layers.

### Request-Path Tests

- mocked tests proving that `search` uses browser-auth headers when the client was built from `browser.json`
- mocked tests proving that `search` still uses the anonymous visitor flow when no browser auth is configured

### Parser Regression Tests

- sanitized authenticated filtered `songs` fixture parses into non-empty typed results
- sanitized authenticated filtered `videos` fixture parses into stable typed results
- existing anonymous/default fixtures remain covered so this fix does not regress the current parser surface

### Local Live Smoke Coverage

Extend the ignored live authenticated test path so it exercises:

- filtered `songs` search
- filtered `videos` search

The live path should continue to rely on the repo-root `browser.json` generated from local `browser.txt`, but no credentials or raw headers should be committed.

## Compatibility Policy

Compatibility remains behavioral against upstream `ytmusicapi` `v1.12.0`, not structural against Python internals.

For this slice, the Rust port should be considered compatible when:

- authenticated clients built from `browser.json` can perform filtered `songs` searches and receive typed non-empty results when upstream returns them
- authenticated clients built from `browser.json` can perform richer filtered `videos` searches through the same public `search` method
- unauthenticated clients continue using the existing anonymous catalog-search path without public API breakage

This slice does not promise full authenticated search parity across every filter, scope, or continuation mode.

## Implementation Constraints

- Keep the crate async-first.
- Do not add a second public search API.
- Reuse existing browser-auth loading and header validation.
- Prefer authenticated search transport automatically when browser auth exists.
- Keep committed fixtures sanitized.
- Use the current local `browser.txt` / `browser.json` setup for live verification during implementation.

## Expected Outcome

After this slice lands, the crate should preserve its existing typed `search` surface while fixing the current authenticated filtered-search gap:

- `search(...with_filter(SearchFilter::Songs))` should return real results for browser-authenticated clients instead of the current no-results payload path
- `search(...with_filter(SearchFilter::Videos))` should use the richer authenticated response path when available
- anonymous clients should keep working without any API change
- the regression suite should pin both the authenticated request path and real authenticated filtered response parsing
