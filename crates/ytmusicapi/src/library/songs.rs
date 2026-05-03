use serde_json::Value;

use crate::{AlbumRef, ArtistRef, Error, LibraryLikeStatus, LibrarySong, Thumbnail};

use super::core::{library_shelf_contents, optional_text, parse_thumbnails, required_text};

pub(crate) fn parse_library_songs_response(response: &Value) -> Result<Vec<LibrarySong>, Error> {
    library_shelf_contents(response)?
        .iter()
        .skip_while(|item| is_leading_non_song_control_row(item))
        .map(parse_library_song)
        .collect()
}

fn parse_library_song(item: &Value) -> Result<LibrarySong, Error> {
    parse_song_list_item(item, "song shelf item").map(|parsed| LibrarySong {
        video_id: parsed.video_id,
        title: parsed.title,
        artists: parsed.artists,
        album: parsed.album,
        duration: parsed.duration,
        thumbnails: parsed.thumbnails,
        like_status: parsed.like_status,
    })
}

pub(crate) struct ParsedSongListItem {
    pub(crate) video_id: String,
    pub(crate) title: String,
    pub(crate) artists: Vec<ArtistRef>,
    pub(crate) album: Option<AlbumRef>,
    pub(crate) duration: Option<String>,
    pub(crate) thumbnails: Vec<Thumbnail>,
    pub(crate) like_status: Option<LibraryLikeStatus>,
}

pub(crate) fn parse_song_list_item(
    item: &Value,
    item_label: &str,
) -> Result<ParsedSongListItem, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(format!(
            "library response missing musicResponsiveListItemRenderer in {item_label}"
        ))
    })?;
    let layout = infer_column_layout(renderer);
    let title_index = layout
        .title_index
        .ok_or_else(|| Error::Parse("library response missing song title".to_owned()))?;
    let title = column_title_text(&flex_columns(renderer)[title_index])
        .ok_or_else(|| Error::Parse("library response missing song title".to_owned()))?;
    let metadata = parse_song_metadata(renderer, &layout);

    Ok(ParsedSongListItem {
        video_id: required_text(renderer, "/playlistItemData/videoId")?,
        title,
        artists: metadata.artists,
        album: metadata.album,
        duration: parse_fixed_duration(renderer).or(metadata.duration),
        thumbnails: parse_thumbnails(renderer)?,
        like_status: parse_like_status(renderer),
    })
}

fn is_leading_non_song_control_row(item: &Value) -> bool {
    item.get("musicResponsiveListItemRenderer")
        .is_some_and(|renderer| {
            renderer.get("playlistItemData").is_none() && !looks_like_song_row(renderer)
        })
}

fn looks_like_song_row(renderer: &Value) -> bool {
    renderer.get("thumbnail").is_some()
        || renderer.get("menu").is_some()
        || renderer.get("fixedColumns").is_some()
        || flex_columns(renderer).iter().any(|column| {
            flex_column_runs(column).iter().any(|run| {
                run.pointer("/navigationEndpoint/watchEndpoint").is_some()
                    || optional_text(run, "/navigationEndpoint/browseEndpoint/browseId").is_some()
                    || optional_text(run, "/text")
                        .is_some_and(|text| looks_like_duration(text.trim()))
            })
        })
}

#[derive(Default)]
struct ColumnLayout {
    title_index: Option<usize>,
    artist_index: Option<usize>,
    album_index: Option<usize>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ColumnSignal {
    Empty,
    Title,
    Artist,
    Album,
    Duration,
    Unknown,
    MixedMetadata,
}

fn infer_column_layout(renderer: &Value) -> ColumnLayout {
    let mut layout = ColumnLayout::default();
    let mut unknown_indexes = Vec::new();

    for (index, column) in flex_columns(renderer).iter().enumerate() {
        match classify_column(column) {
            ColumnSignal::Title => {
                layout.title_index.get_or_insert(index);
            }
            ColumnSignal::Artist => {
                layout.artist_index.get_or_insert(index);
            }
            ColumnSignal::Album => {
                layout.album_index.get_or_insert(index);
            }
            ColumnSignal::Unknown => unknown_indexes.push(index),
            ColumnSignal::Empty | ColumnSignal::Duration | ColumnSignal::MixedMetadata => {}
        };
    }

    if layout.title_index.is_none() {
        layout.title_index = take_title_index(&mut unknown_indexes, &layout);
    }
    if let Some(title_index) = layout.title_index {
        remove_index(&mut unknown_indexes, title_index);
    }

    if layout.artist_index.is_none() {
        layout.artist_index = take_artist_index(&mut unknown_indexes, &layout);
    }
    if let Some(artist_index) = layout.artist_index {
        remove_index(&mut unknown_indexes, artist_index);
    }

    if layout.album_index.is_none() {
        layout.album_index = take_album_index(&mut unknown_indexes, &layout);
    }

    layout
}

fn classify_column(column: &Value) -> ColumnSignal {
    let runs = flex_column_runs(column);
    if runs.is_empty() || column_text(column).is_none() {
        return ColumnSignal::Empty;
    }
    if runs.iter().any(has_watch_endpoint) {
        return ColumnSignal::Title;
    }

    let mut saw_artist = false;
    let mut saw_album = false;
    let mut saw_non_duration_text = false;

    for run in runs {
        let Some(text) = optional_text(run, "/text") else {
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() || trimmed == "•" {
            continue;
        }

        if let Some(browse_id) = optional_text(run, "/navigationEndpoint/browseEndpoint/browseId") {
            if is_album_run(run, &browse_id) {
                saw_album = true;
            } else {
                saw_artist = true;
            }
            continue;
        }

        if !looks_like_duration(trimmed) {
            saw_non_duration_text = true;
        }
    }

    match (saw_artist, saw_album) {
        (true, false) => ColumnSignal::Artist,
        (false, true) => ColumnSignal::Album,
        (true, true) => ColumnSignal::MixedMetadata,
        (false, false) if saw_non_duration_text => ColumnSignal::Unknown,
        (false, false) => ColumnSignal::Duration,
    }
}

fn take_title_index(unknown_indexes: &mut Vec<usize>, layout: &ColumnLayout) -> Option<usize> {
    if unknown_indexes.len() == 1 {
        return unknown_indexes.pop();
    }

    if let Some(boundary) = layout.artist_index.or(layout.album_index)
        && let Some(index) = take_unique_matching(unknown_indexes, |index| index < boundary)
    {
        return Some(index);
    }

    take_first_index(unknown_indexes)
}

fn take_artist_index(unknown_indexes: &mut Vec<usize>, layout: &ColumnLayout) -> Option<usize> {
    if unknown_indexes.len() == 1 {
        return unknown_indexes.pop();
    }

    if let Some(album_index) = layout.album_index
        && let Some(index) = take_unique_matching(unknown_indexes, |index| index < album_index)
    {
        return Some(index);
    }

    layout
        .title_index
        .and_then(|_| take_first_index(unknown_indexes))
}

fn take_album_index(unknown_indexes: &mut Vec<usize>, layout: &ColumnLayout) -> Option<usize> {
    if unknown_indexes.len() == 1 {
        return unknown_indexes.pop();
    }

    if let Some(artist_index) = layout.artist_index
        && let Some(index) = take_unique_matching(unknown_indexes, |index| index > artist_index)
    {
        return Some(index);
    }

    match (layout.title_index, layout.artist_index) {
        (Some(_), Some(_)) => take_first_index(unknown_indexes),
        _ => None,
    }
}

fn take_unique_matching(
    indexes: &mut Vec<usize>,
    predicate: impl Fn(usize) -> bool,
) -> Option<usize> {
    let matches: Vec<usize> = indexes
        .iter()
        .copied()
        .filter(|index| predicate(*index))
        .collect();
    if matches.len() != 1 {
        return None;
    }

    let matched = matches[0];
    remove_index(indexes, matched);
    Some(matched)
}

fn remove_index(indexes: &mut Vec<usize>, target: usize) {
    indexes.retain(|index| *index != target);
}

fn take_first_index(indexes: &mut Vec<usize>) -> Option<usize> {
    (!indexes.is_empty()).then(|| indexes.remove(0))
}

struct ParsedSongMetadata {
    artists: Vec<ArtistRef>,
    album: Option<AlbumRef>,
    duration: Option<String>,
}

fn parse_song_metadata(renderer: &Value, layout: &ColumnLayout) -> ParsedSongMetadata {
    let mut parsed = ParsedSongMetadata {
        artists: Vec::new(),
        album: None,
        duration: None,
    };

    for (index, column) in flex_columns(renderer).iter().enumerate() {
        if Some(index) == layout.title_index {
            continue;
        }

        for run in flex_column_runs(column) {
            let Some(text) = optional_text(run, "/text") else {
                continue;
            };
            let trimmed = text.trim();
            if trimmed.is_empty() || trimmed == "•" {
                continue;
            }

            if let Some(browse_id) =
                optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
            {
                if parsed.album.is_none() && is_album_run(run, &browse_id) {
                    parsed.album = Some(AlbumRef {
                        id: browse_id,
                        name: text,
                    });
                } else {
                    parsed.artists.push(ArtistRef {
                        id: browse_id,
                        name: text,
                    });
                }
                continue;
            }

            if parsed.duration.is_none() && looks_like_duration(trimmed) {
                parsed.duration = Some(trimmed.to_owned());
            } else if Some(index) == layout.artist_index {
                parsed.artists.push(ArtistRef {
                    id: String::new(),
                    name: text,
                });
            }
        }

        if parsed.album.is_none()
            && Some(index) == layout.album_index
            && let Some(name) = column_text(column)
        {
            parsed.album = Some(AlbumRef {
                id: String::new(),
                name,
            });
        }
    }

    parsed
}

fn parse_fixed_duration(renderer: &Value) -> Option<String> {
    optional_text(
        renderer,
        "/fixedColumns/0/musicResponsiveListItemFixedColumnRenderer/text/runs/0/text",
    )
}

fn flex_columns(renderer: &Value) -> &[Value] {
    renderer
        .pointer("/flexColumns")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub(crate) fn flex_column_runs(column: &Value) -> &[Value] {
    column
        .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub(crate) fn column_title_text(column: &Value) -> Option<String> {
    let mut parts = meaningful_run_texts(column);
    while parts.len() > 1 && looks_like_title_badge(parts[0].trim()) {
        parts.remove(0);
    }

    let text = parts.concat();
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn column_text(column: &Value) -> Option<String> {
    let text = meaningful_run_texts(column).concat();
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn meaningful_run_texts(column: &Value) -> Vec<String> {
    flex_column_runs(column)
        .iter()
        .filter_map(|run| optional_text(run, "/text"))
        .filter(|text| {
            let trimmed = text.trim();
            !trimmed.is_empty() && trimmed != "•"
        })
        .collect()
}

fn has_watch_endpoint(run: &Value) -> bool {
    run.pointer("/navigationEndpoint/watchEndpoint").is_some()
        || run
            .pointer("/navigationEndpoint/watchPlaylistEndpoint")
            .is_some()
}

fn is_album_run(run: &Value, browse_id: &str) -> bool {
    matches!(
        optional_text(
            run,
            "/navigationEndpoint/browseEndpoint/browseEndpointContextSupportedConfigs/browseEndpointContextMusicConfig/pageType",
        )
        .as_deref(),
        Some("MUSIC_PAGE_TYPE_ALBUM") | Some("MUSIC_PAGE_TYPE_AUDIOBOOK")
    ) || browse_id.starts_with("MPRE")
        || browse_id.contains("release_detail")
}

fn looks_like_title_badge(value: &str) -> bool {
    matches!(value, "E" | "Explicit" | "EXPLICIT")
}

pub(crate) fn looks_like_duration(value: &str) -> bool {
    let mut parts = value.split(':');
    let Some(first) = parts.next() else {
        return false;
    };
    if first.is_empty() || !first.chars().all(|ch| ch.is_ascii_digit()) {
        return false;
    }

    let mut count = 1;
    for part in parts {
        if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
            return false;
        }
        count += 1;
    }

    count >= 2
}

fn parse_like_status(renderer: &Value) -> Option<LibraryLikeStatus> {
    match optional_text(
        renderer,
        "/menu/menuRenderer/topLevelButtons/0/likeButtonRenderer/likeStatus",
    )
    .as_deref()
    {
        Some("LIKE") => Some(LibraryLikeStatus::Like),
        Some("INDIFFERENT") => Some(LibraryLikeStatus::Indifferent),
        Some("DISLIKE") => Some(LibraryLikeStatus::Dislike),
        _ => None,
    }
}
