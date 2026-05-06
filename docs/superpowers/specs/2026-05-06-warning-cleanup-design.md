# Warning Cleanup Design

## Summary

This phase removes the obsolete dead-code leftovers from the liked-songs / saved-episodes wrapper migration and cleans the warning-producing leftovers in the recently added search-continuation implementation.

The phase covers:

- removing the unused `LikedSongs` model struct
- removing the unused `SavedEpisodes` model struct
- fixing the current `needless_borrow` warnings in `client.rs`
- fixing the small local clippy warning in `search/parse.rs`

This phase does not change runtime behavior or public feature scope. It is cleanup only.

## Goals

- Remove the two known dead-code warnings from `model/library.rs`.
- Keep `LikedSongsPage` and `SavedEpisodesPage` as the only public wrappers for those surfaces.
- Clean the warning-producing leftovers in the search-continuation code that was just added.
- Keep the cleanup tightly scoped to the touched migration and search-continuation area.

## Non-Goals

- Broader crate-wide warning cleanup
- Refactoring unrelated library modules such as `library/core.rs`
- API changes beyond deleting obsolete unused structs
- Functional behavior changes to search, library, or auth flows

## Scope

### Model Cleanup

Delete the obsolete structs from `crates/ytmusicapi/src/model/library.rs`:

```rust
pub struct LikedSongs { ... }
pub struct SavedEpisodes { ... }
```

The active public wrappers remain:

```rust
pub struct LikedSongsPage { ... }
pub struct SavedEpisodesPage { ... }
```

No compatibility shim is needed because the page-aware wrappers already replaced the old types in the public API.

### Search-Continuation Warning Cleanup

Clean the warning-producing leftovers in the recent search-continuation area only:

- remove the current `needless_borrow` clippy warnings in `crates/ytmusicapi/src/client.rs`
- simplify the small nested branch in `crates/ytmusicapi/src/search/parse.rs` that currently triggers a local clippy warning

These changes should stay mechanical and local. They should not alter request behavior, parser behavior, or test expectations.

## File Boundaries

The expected files in scope are:

- `crates/ytmusicapi/src/model/library.rs`
- `crates/ytmusicapi/src/client.rs`
- `crates/ytmusicapi/src/search/parse.rs`

Other files should remain untouched unless a narrowly necessary follow-on fix is required to keep the build green.

## Error Handling And Behavior

This phase should not change error behavior, transport behavior, parsing behavior, or public return shapes.

The only user-visible effect should be a quieter build and a cleaner model layer.

## Testing And Verification

Verification should be explicit and limited to the cleanup goal:

- run `cargo test --workspace -q`
- run `cargo clippy -p ytmusicapi --all-targets --all-features`

Success means:

- the `LikedSongs` and `SavedEpisodes` dead-code warnings are gone
- the targeted search-continuation warnings are gone
- tests still pass

If unrelated warnings remain elsewhere in the crate after this cleanup, they should be reported explicitly and left alone.

## Deferred Work

The following remain intentionally out of scope:

- `library/core.rs` clippy suggestions
- unrelated clippy cleanup elsewhere in the crate
- cleanup of empty or noisy history
- any new feature work or parser expansion
