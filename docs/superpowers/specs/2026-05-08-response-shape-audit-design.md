# Response Shape Audit Design

## Summary

This phase audits the actual JSON response shapes for every currently implemented `YtMusic` API call method in the Rust crate, expands the raw fixture corpus with live captures, corrects parser and public model assumptions based on evidence, and then removes the temporary response-capture plumbing from the library code.

The goal is not just to collect more payloads. The goal is to replace assumption-driven typing with evidence-backed typing:

- identify fields currently modeled as `Option<T>` that are consistently present across the required observed variants
- identify fields currently treated as always present that are missing in some observed variants or items
- identify paths whose value type or container shape varies across methods, variants, or item rows
- preserve the intentional public API style of this Rust port rather than mirroring raw upstream JSON

This work covers the Rust methods that already exist today, including continuation methods and browser-authenticated surfaces where implemented. It also includes applying the model and parser fixes uncovered by the audit.

## Goals

- Capture raw `serde_json::Value` payloads for every implemented API call method in the Rust crate.
- Cover a sufficiently diverse set of response-shape variants for each method rather than a single happy-path payload.
- Build an evidence-backed audit over the committed raw fixtures.
- Tighten public models only when the observed corpus supports doing so.
- Relax parser assumptions and public typing where the observed corpus shows fields are absent or unstable.
- Add lasting fixture-backed regression tests.
- Remove the temporary capture plumbing from project code after the fixtures and fixes are committed.

## Non-Goals

- Auditing the full upstream Python `ytmusicapi` surface that is not yet implemented in this Rust crate.
- Keeping raw-response recording logic in the long-term library codebase.
- Exposing raw response payloads as a stable public API.
- Building a generic schema-inference system intended for reuse outside this repo.
- Solving dynamic signature timestamp discovery as part of this project.

## Scope

The audit targets the currently implemented Rust client methods:

- `search`
- `search_continuation`
- `get_watch_playlist`
- `get_watch_playlist_continuation`
- `get_song`
- `get_library_playlists`
- `get_library_playlists_continuation`
- `get_account_info`
- `get_library_artists`
- `get_library_artists_continuation`
- `get_library_albums`
- `get_library_albums_continuation`
- `get_library_subscriptions`
- `get_library_subscriptions_continuation`
- `get_library_channels`
- `get_library_channels_continuation`
- `get_library_podcasts`
- `get_library_podcasts_continuation`
- `get_library_songs`
- `get_library_songs_continuation`
- `get_liked_songs`
- `get_liked_songs_continuation`
- `get_saved_episodes`
- `get_saved_episodes_continuation`

This project includes both anonymous and authenticated variants when the current Rust surface supports them. Continuation methods are treated as separate capture targets because their top-level payload structure may differ materially from first-page responses.

## Recommended Approach

Use a Rust-first workflow:

1. add temporary internal capture hooks around the shared JSON transport seams already used by the client
2. drive live captures through the actual Rust `YtMusic` methods so the collected corpus matches the implemented public surface
3. commit the resulting raw fixtures
4. run an audit pass that compares observed field presence and shape stability against the current parser and model assumptions
5. fix the parser and typed models
6. remove the temporary capture hooks from the library code

This is preferred over a Python-first harness because the Rust client already has stable shared transport seams and the target of the audit is the Rust implementation, not an abstract upstream schema.

## Architecture

The work is divided into four bounded components.

### 1. Temporary Raw Capture Instrumentation

Add a narrow internal hook at the shared JSON transport layer so a dev-only capture harness can persist the raw response body before parsing.

The preferred seams are the existing helpers in `client.rs`:

- `search_raw_transport`
- `post_authenticated_json`
- by extension, the endpoint-specific wrappers `post_browse`, `post_next`, and `post_player`

The hook is temporary implementation scaffolding. It exists only to collect the fixture corpus needed for this audit and must be removed after the fixture and parser/model work is complete.

### 2. Live Capture Harness

Add a repo-local developer harness that exercises every implemented client method and writes committed raw JSON fixtures.

The harness should:

- invoke the real Rust `YtMusic` methods, not private parser entrypoints
- store first-page and continuation captures separately
- distinguish anonymous and authenticated captures where both are supported
- record capture metadata in filenames or neighboring manifest data rather than embedding extra fields into the response payload itself
- treat methods with no available continuation at capture time as coverage gaps rather than as successful continuation coverage

For `get_song`, the request used during capture should force `signatureTimestamp = 20577`. This ensures the captured player payloads reflect one consistent request shape and avoids turning this project into a signature timestamp discovery effort.

### 3. Shape Audit

Build a focused audit tool over the committed raw fixtures that walks `serde_json::Value` payloads and records observations by JSON path.

The audit should classify fields and paths at minimum as:

- always present in the required observed corpus for that method/variant
- present only in some variants
- missing in some rows within the same item list
- type-unstable across captures
- not yet evidenced enough to justify model tightening

The audit should reason at two levels:

- top-level and nested response sections
- repeated item rows inside arrays, because many parser assumptions fail at the item level rather than at the response root

### 4. Parser And Model Fix Loop

Use the audit results to update:

- typed public response models
- response parsing logic
- fixture-backed tests

Public typing changes should stay conservative. Tighten `Option<T>` to required only when the required coverage matrix for that method shows stable presence. When the observed corpus shows genuine absence or shape variability, preserve optionality or parse the value behind a more resilient internal boundary.

## Coverage Matrix

The project should define coverage in terms of shape diversity, not raw fixture count. Each method needs a small matrix of required variants and optional variants that are known to materially affect response shape.

### Required Coverage Dimensions

- first page vs continuation
- anonymous vs authenticated when both are supported
- empty/sparse account state vs populated state where the account naturally exposes both
- method-specific mode switches that change payload shape

### Method-Specific Expectations

The current known minimum matrix is:

- `search`: default mixed search, multiple filters, authenticated search where supported, first-page and continuation payloads
- `get_watch_playlist`: normal playlist, radio, shuffle, and continuation
- `get_song`: multiple song/player payloads with different observed stream and metadata shapes, all captured with `signatureTimestamp = 20577`
- browser-auth library/account methods: populated responses where available, plus sparse or empty-state responses when the account naturally returns them

The harness should not stop at one successful payload per method. A method is sufficiently covered only when its known shape axes are represented well enough to support model decisions with evidence. If a variant cannot be captured from the available account state, that gap must be recorded explicitly.

## Data Flow

The intended workflow is:

1. start from the real Rust client method
2. perform the live HTTP request through the existing transport helper
3. intercept and persist the raw JSON response as a fixture
4. parse the response into the existing typed model
5. run the audit over the committed fixture corpus
6. update parser and model code based on the audit results
7. remove the temporary capture hook from the library code
8. keep the raw fixtures and regression tests as the lasting evidence base

This keeps the stable library surface clean while preserving the artifacts needed to justify and verify the parser/model behavior.

## Audit Rules

The audit should apply these decision rules:

- A field currently modeled as `Option<T>` should only become required if it is consistently present across the method's required coverage matrix.
- A field currently treated as required should become optional or parser-resilient if the observed corpus shows legitimate absence in any required variant.
- Type instability should be treated as a parser design issue even if all values are technically present.
- Evidence gaps must remain explicit. Lack of capture is not evidence of absence or presence.

The output should separate:

- confirmed model mismatch
- parser fragility caused by missing or variant-only fields
- unstable or mixed-type paths
- unverified assumptions due to incomplete coverage

## Error Handling And Gaps

Capture failures should be recorded as missing evidence, not silently skipped.

Expected gap categories include:

- no continuation token available for the captured first page
- account lacks data for a surface or variant
- live request failed transiently
- a known shape axis could not be reproduced from the available environment

The reporting should distinguish:

- confirmed optionality from observed missing fields
- incomplete coverage where no claim should be made yet

## Testing Strategy

The long-lived verification after cleanup should be fixture-backed.

- parser tests should load the committed raw fixtures and assert the corrected typed outputs or targeted field expectations
- transport tests should continue verifying request construction where appropriate
- the audit tool or its expectations should run against the committed fixture corpus to guard against accidental reintroduction of unsupported field assumptions

The temporary capture instrumentation itself should not remain part of the final verification surface.

## Cleanup Requirement

Temporary raw-capture logic added to support live fixture collection must be removed from the project code once:

- the raw fixture corpus is committed
- the audit-backed parser/model fixes are committed
- lasting regression tests cover the intended behavior

The repo should finish this project with:

- a larger committed raw fixture corpus
- stronger parser and model tests
- corrected parser and public model assumptions
- no permanent capture hook in the production library code

## Risks And Tradeoffs

- Live captures can be account-dependent, especially for browser-auth library surfaces. The design handles this by recording coverage gaps explicitly instead of pretending complete characterization.
- Over-tightening public models based on a thin corpus would create regressions. The audit rules intentionally require required-coverage evidence before converting optional fields into required fields.
- Keeping capture logic in the final code would add maintenance cost and muddy the library boundary. The temporary-instrumentation cleanup requirement avoids that.

## Success Criteria

This project is successful when:

- every currently implemented Rust API method has committed raw fixtures for the required reachable variants
- the fixture corpus includes materially different shapes rather than one capture per method
- the audit identifies concrete model and parser mismatches
- the code applies those corrections with fixture-backed regression coverage
- the temporary raw-capture plumbing has been removed from the library code
