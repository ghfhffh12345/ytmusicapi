# Browser Auth, Library Playlists, And Workspace Restructure Design

Date: 2026-05-01
Upstream baseline: `sigma67/ytmusicapi` `v1.12.0`
Status: Approved design for next implementation slice

## Summary

The next slice extends the crate from anonymous public search into authenticated browser-session usage. It converts the repository into a virtual Cargo workspace, adds browser-header authentication support to the library, introduces one authenticated proving endpoint (`get_library_playlists`), and adds a small CLI crate that converts pasted Firefox request headers into upstream-compatible `browser.json`.

This slice keeps scope intentionally narrow. It does not attempt OAuth, account mutation, or a general auth management tool. The library remains the source of truth for authentication semantics; the CLI only produces the file format that the library consumes.

## Goals

- Convert the repository into a virtual Cargo workspace with separate library and CLI crates.
- Preserve the library crate name `ytmusicapi`.
- Add browser-authenticated client construction using an upstream-compatible `browser.json` file.
- Implement authenticated `get_library_playlists` as the proving endpoint.
- Add a simple CLI that reads Firefox-copied headers from `stdin`, validates them, and writes `browser.json` in the current directory.
- Reuse well-established crates and keep the public API idiomatic Rust.

## Non-Goals

- OAuth support.
- Authenticated search scope expansion.
- Additional authenticated endpoints beyond `get_library_playlists`.
- Playlist or library mutation.
- A multi-mode auth CLI with file input, browser automation, or an interactive wizard.
- CI dependence on live authenticated YouTube Music traffic.

## Scope

This slice includes four tightly related changes:

1. Repository restructure into a virtual workspace.
2. Browser-auth file loading and validation in the library.
3. One authenticated library-read endpoint: `get_library_playlists`.
4. A thin CLI crate that converts pasted Firefox headers into `browser.json`.

Anything outside those boundaries is explicitly deferred.

## Workspace Architecture

The repository root should become a virtual workspace manifest. The new layout should be:

- `Cargo.toml` at the repo root as a workspace manifest
- `crates/ytmusicapi` for the library crate
- `crates/ytmusicapi-cli` for the executable crate

The library package name remains `ytmusicapi`. The CLI package and binary should use a distinct name such as `ytmusicapi-cli`.

Shared developer commands such as `cargo test`, `cargo fmt`, and `cargo clippy --workspace --all-targets --all-features` should run from the workspace root.

## Library Architecture

The library should grow by adding bounded modules rather than merging auth logic into the existing search code. The target split is:

- `client`: public `YtMusic` constructors and request orchestration
- `auth`: browser-auth file model, loading, normalization, and validation
- `library`: authenticated library endpoint request/response handling
- existing `search`, `model`, and `error` modules extended where needed

This keeps browser auth isolated from endpoint parsing and leaves a clean path for future OAuth support without reshaping the whole crate.

## Public API Direction

The unauthenticated constructor path should remain intact. This slice adds an authenticated construction path that accepts a filesystem path to a `browser.json`-compatible file and builds an authenticated client from it.

The Rust API should stay idiomatic:

- file-based browser auth is the first supported authenticated entrypoint
- validated typed models exist internally even if the public entrypoint is path-based
- constructors fail early and clearly on missing files, malformed JSON, or invalid browser headers

The library should not expose public API shapes that imply OAuth or other auth modes already exist.

## Browser Auth File Compatibility

The accepted file format should match upstream `ytmusicapi` browser-auth JSON closely enough that a valid `browser.json` created for upstream `v1.12.0` can be consumed here.

Validation should be strict about required browser-auth headers, including the fields needed for authenticated YouTube Music requests. At minimum, this slice should treat the following as required unless implementation evidence proves a narrower set is sufficient:

- `Authorization`
- `Cookie`
- `X-Goog-AuthUser`
- `x-origin`
- `Content-Type`
- `Accept`

Header-name normalization should be explicit so equivalent casing is accepted, but the stored JSON output should use a stable canonical shape.

## CLI Behavior

The CLI crate should do one job:

1. Read raw Firefox-copied request headers from `stdin`.
2. Parse them into header key/value pairs.
3. Validate the required browser-auth header set using the same core rules as the library.
4. Write `browser.json` into the current directory.

The CLI should not print JSON to `stdout` as its primary output, should not accept alternate input files in this slice, and should not attempt to perform authenticated requests itself.

Errors should be explicit when:

- the pasted input is empty
- the input is not parseable as request headers
- required headers are missing
- duplicate or malformed header lines make the result ambiguous
- the output file cannot be written

## Authenticated Request Flow

Authenticated use should follow this flow:

1. The user captures Firefox request headers and saves them, or pipes them to the CLI.
2. The CLI validates the pasted headers and writes `browser.json`.
3. The caller constructs `YtMusic` from the `browser.json` path.
4. The library loads and validates the JSON into a typed internal browser-auth config.
5. Authenticated requests merge those validated headers into the existing transport pipeline.
6. Endpoint-specific logic sends authenticated YouTube Music requests and parses typed results.

Anonymous bootstrap logic from the search slice should remain available for anonymous endpoints, but authenticated calls must use the explicit auth-header path rather than pretending anonymous bootstrap is sufficient.

## Proving Endpoint: `get_library_playlists`

`get_library_playlists` should be the only new endpoint in this slice. It is a good proving endpoint because it clearly requires authentication and exercises the library-specific `browse` flow without forcing a very broad parser surface.

The Rust API should return typed playlist entries rather than raw JSON. The returned model should focus on the stable, meaningful subset of upstream `v1.12.0` playlist-library data, such as:

- playlist title
- playlist or browse identifier
- owner/author when present
- item count when present
- thumbnails

If the upstream endpoint supports continuations, this slice should choose and document one explicit behavior. The design choice here is first-page-only behavior unless continuation support falls out naturally from the existing architecture with low additional complexity.

## Error Handling

The public error model should remain `thiserror`-based and gain auth- and file-related variants as needed. Distinct failure classes should remain visible to callers:

- file I/O failure while loading `browser.json`
- browser-auth JSON decode failure
- browser-auth validation failure for missing or malformed required headers
- authenticated HTTP transport failure
- non-success authenticated HTTP status
- semantic parse failure when upstream response structure no longer matches supported models
- CLI parse/validation/output failures

Validation failures should happen before sending any authenticated request.

## Testing Strategy

This slice should use deterministic local tests by default:

- library unit tests for browser-auth file parsing, normalization, and validation
- constructor tests for authenticated and unauthenticated client setup
- CLI tests for `stdin` header parsing and `browser.json` emission
- mocked or fixture-based tests for `get_library_playlists` request shape, auth-header propagation, and typed response parsing

Live authenticated network tests must stay out of default CI.

### Local Testing Input

The repository-root `browser.txt` file provided by the user should be treated as the local testing source for Firefox-copied headers during implementation of this slice. It is suitable for local validation and manual testing, but it must not become a committed fixture if it contains live credentials or session data.

If authenticated response fixtures are captured from a real session, sensitive headers and cookie material must be stripped before anything is committed.

## Compatibility Policy

Compatibility in this slice is behavioral, not structural. For upstream `v1.12.0`, this means:

- a valid upstream-style `browser.json` works as an authenticated input
- the CLI produces a `browser.json` shape the library accepts
- authenticated `get_library_playlists` behaves equivalently within the documented subset

The Rust crate does not need to mirror upstream Python module boundaries or setup UX beyond the explicitly chosen file-based browser-auth flow.

## Implementation Constraints

- Keep the library crate async-first.
- Reuse established crates where they clearly reduce risk.
- Avoid duplicating browser-auth validation logic between the library and CLI.
- Keep the CLI thin; it should delegate shared parsing and validation to library-owned code where practical.
- Keep this slice focused enough for one implementation plan: workspace conversion, browser auth, one authenticated read endpoint, and one narrow CLI.

## Expected Outcome

After this slice lands, the repository should provide:

- a clean two-crate workspace layout
- a library that supports both anonymous search and browser-authenticated client construction
- a typed authenticated `get_library_playlists` endpoint
- a small CLI that turns pasted Firefox headers into `browser.json`

That outcome establishes the auth foundation for later authenticated endpoints without prematurely taking on OAuth or a broad CLI surface.
