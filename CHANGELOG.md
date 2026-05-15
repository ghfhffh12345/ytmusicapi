# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog,
and this project adheres to Semantic Versioning 2.0.0.

## [Unreleased]

## [0.1.0] - 2026-05-15

### Added
- Initial Rust workspace release with the `ytmusicapi` library crate and the `ytmusicapi-cli` browser header converter.
- Typed search queries and typed search result models for mixed, song, video, album, artist, and playlist search surfaces.
- Explicit continuation-driven pagination with shared `Page<T, C>` responses and method-specific continuation token types.
- Browser-authenticated client construction from `browser.json` plus authenticated search and library request support.
- Typed library client methods for playlists, artists, albums, subscriptions, channels, podcasts, songs, liked songs, saved episodes, and account info.
- Watch playlist and continuation support with typed watch queries and watch track models.
- Typed `get_song` support for player payloads, including video details, playability status, streaming data, and microformat fields.

### Fixed
- Parser handling for varying YouTube Music response shapes across search, library, watch, and song payloads, backed by fixture-based regression coverage.
