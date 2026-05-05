use serde_json::Value;

use crate::{
    Error, SearchFilter,
    model::{
        common::{AlbumRef, ArtistRef, Thumbnail},
        search::{
            AlbumResult, ArtistResult, PlaylistResult, ProfileResult, SearchResult,
            SearchResultType, SongResult, VideoResult,
        },
    },
};

pub fn parse_search_response(
    response: &Value,
    filter: Option<SearchFilter>,
) -> Result<crate::Page<SearchResult>, Error> {
    let tabs = required_array_at(response, "/contents/tabbedSearchResultsRenderer/tabs")?;
    if tabs.is_empty() {
        return Ok(crate::Page {
            items: Vec::new(),
            continuation: None,
        });
    }

    let sections = required_array_at(
        response,
        "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
    )?;

    let items = match filter {
        None => parse_default_mixed_sections(sections)?,
        Some(SearchFilter::Songs) => parse_filtered_sections(sections, SearchFilter::Songs)?,
        Some(SearchFilter::Videos) => parse_filtered_sections(sections, SearchFilter::Videos)?,
        Some(SearchFilter::Albums) => parse_filtered_sections(sections, SearchFilter::Albums)?,
        Some(SearchFilter::Artists) => parse_filtered_sections(sections, SearchFilter::Artists)?,
        Some(SearchFilter::Playlists) => {
            parse_filtered_sections(sections, SearchFilter::Playlists)?
        }
    };

    Ok(crate::Page {
        items,
        continuation: extract_search_continuation(response, sections)?,
    })
}

pub(crate) fn parse_search_continuation_response(
    response: &Value,
) -> Result<crate::Page<crate::SearchResult>, Error> {
    let shelf = response
        .pointer("/continuationContents/musicShelfContinuation")
        .ok_or_else(|| {
            Error::Parse("missing continuationContents.musicShelfContinuation".to_owned())
        })?;

    let contents = shelf
        .get("contents")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::Parse("missing continuation contents array".to_owned()))?;

    let items = contents
        .iter()
        .map(parse_search_continuation_item)
        .collect::<Result<Vec<_>, _>>()?;

    let continuation = shelf
        .pointer("/continuations/0/nextContinuationData/continuation")
        .and_then(Value::as_str)
        .map(crate::ContinuationToken::new)
        .transpose()?;

    Ok(crate::Page {
        items,
        continuation,
    })
}

fn parse_search_continuation_item(item: &Value) -> Result<SearchResult, Error> {
    let renderer = required_value_at(item, "/musicResponsiveListItemRenderer")?;
    let metadata_runs = required_array_at(
        renderer,
        "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs",
    )?;
    let metadata_parts = non_separator_runs(metadata_runs);
    let leading_text = metadata_parts
        .first()
        .map(|run| required_text(run, "/text"))
        .transpose()?;

    match leading_text.as_deref() {
        Some(
            "Song" | "Video" | "Album" | "Single" | "EP" | "Artist" | "Profile" | "Playlist"
            | "Episode" | "Podcast",
        ) => parse_shelf_item(item, None),
        _ => parse_filtered_search_continuation_item(item, renderer, &metadata_parts),
    }
}

fn parse_filtered_search_continuation_item(
    item: &Value,
    renderer: &Value,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    // Continuation rows omit the explicit filter context from the request contract, so the parser
    // infers the filtered row kind from payload shape only:
    // - song/video rows carry a playable video id
    // - playlist/album/artist rows carry browse ids
    // - album rows still keep an explicit type label ("Album"/"Single"/"EP")
    // - video rows expose a views count, while song rows expose album linkage or just duration
    let leading_text = metadata_parts
        .first()
        .map(|run| required_text(run, "/text"))
        .transpose()?;
    let has_video_id = required_video_id(renderer).is_ok();
    let browse_id = renderer
        .pointer("/navigationEndpoint/browseEndpoint/browseId")
        .and_then(Value::as_str);
    let has_browse_id = browse_id.is_some();
    // Artist continuations can omit subscriber/audience text, so keep channel-style browse ids
    // out of the playlist fallback.
    let has_artist_browse_id = browse_id.is_some_and(|browse_id| browse_id.starts_with("UC"));
    let has_views = metadata_parts.iter().any(|part| {
        required_text(part, "/text")
            .map(|text| text.to_ascii_lowercase().contains("views"))
            .unwrap_or(false)
    });
    let has_subscribers_or_audience = metadata_parts.iter().any(|part| {
        required_text(part, "/text")
            .map(|text| {
                let text = text.to_ascii_lowercase();
                text.contains("subscriber") || text.contains("monthly audience")
            })
            .unwrap_or(false)
    });
    let has_album_link = metadata_parts.iter().skip(1).any(|part| {
        part.pointer("/navigationEndpoint/browseEndpoint/browseId")
            .and_then(Value::as_str)
            .is_some()
    });
    let trailing_text = metadata_parts
        .last()
        .and_then(|part| required_text(part, "/text").ok());

    let inferred_filter = match leading_text.as_deref() {
        Some("Album" | "Single" | "EP") => SearchFilter::Albums,
        _ if has_video_id && has_views => SearchFilter::Videos,
        _ if has_video_id
            && (has_album_link || trailing_text.as_deref().is_some_and(looks_like_duration)) =>
        {
            SearchFilter::Songs
        }
        _ if has_browse_id && has_subscribers_or_audience => SearchFilter::Artists,
        _ if has_artist_browse_id => SearchFilter::Artists,
        _ if has_browse_id => SearchFilter::Playlists,
        _ => {
            return Err(Error::Parse(
                "unable to infer filtered continuation item type from search payload".to_owned(),
            ));
        }
    };

    parse_filtered_shelf_item(item, None, inferred_filter)
}

fn parse_default_mixed_sections(sections: &[Value]) -> Result<Vec<SearchResult>, Error> {
    let mut results = Vec::new();
    for section in sections {
        if let Some(card) = section.get("musicCardShelfRenderer") {
            results.push(parse_top_result(card)?);
        }

        if let Some(shelf) = section.get("musicShelfRenderer") {
            let category = optional_runs_text_at(shelf, "/title/runs");
            for item in required_array_at(shelf, "/contents")? {
                results.push(parse_shelf_item(item, category.clone())?);
            }
        }
    }

    Ok(results)
}

fn parse_filtered_sections(
    sections: &[Value],
    filter: SearchFilter,
) -> Result<Vec<SearchResult>, Error> {
    let mut results = Vec::new();
    for section in sections {
        if let Some(shelf) = section.get("musicShelfRenderer") {
            let category = optional_runs_text_at(shelf, "/title/runs");
            for item in required_array_at(shelf, "/contents")? {
                results.push(parse_filtered_shelf_item(item, category.clone(), filter)?);
            }
        }
    }

    Ok(results)
}

fn extract_search_continuation(
    response: &Value,
    sections: &[Value],
) -> Result<Option<crate::ContinuationToken>, Error> {
    for path in [
        "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/continuations/0/nextContinuationData/continuation",
    ] {
        if let Some(token) = response.pointer(path).and_then(Value::as_str) {
            return crate::ContinuationToken::new(token).map(Some);
        }
    }

    for section in sections {
        if let Some(shelf) = section.get("musicShelfRenderer") {
            if let Some(token) = shelf
                .pointer("/continuations/0/nextContinuationData/continuation")
                .and_then(Value::as_str)
            {
                return crate::ContinuationToken::new(token).map(Some);
            }
        }
    }

    Ok(None)
}

fn parse_top_result(card: &Value) -> Result<SearchResult, Error> {
    let title = required_runs_text_at(card, "/title/runs")?;
    let title_runs = required_array_at(card, "/title/runs")?;
    let subtitle_runs = required_array_at(card, "/subtitle/runs")?;
    let subtitle_parts = non_separator_runs(subtitle_runs);
    let result_kind = subtitle_parts
        .first()
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing top result type label".to_owned()))?;

    match result_kind.as_str() {
        "Artist" => {
            let artist_run = title_runs
                .first()
                .ok_or_else(|| Error::Parse("search response missing /title/runs/0".to_owned()))?;
            let subscribers = subtitle_parts
                .get(1)
                .map(|run| required_text(run, "/text"))
                .transpose()?
                .and_then(|value| value.split_whitespace().next().map(str::to_owned))
                .ok_or_else(|| {
                    Error::Parse("search response missing top result subscribers".to_owned())
                })?;

            Ok(SearchResult::Artist(ArtistResult {
                category: Some("Top result".to_owned()),
                result_type: SearchResultType::Artist,
                artist: None,
                artists: vec![parse_artist_ref(artist_run)?],
                subscribers: Some(subscribers),
                browse_id: None,
                radio_id: None,
                shuffle_id: None,
                thumbnails: parse_thumbnails(card)?,
            }))
        }
        "Song" => parse_song_result(card, Some("Top result".to_owned()), title, &subtitle_parts),
        "Video" => parse_video_result(card, Some("Top result".to_owned()), title, &subtitle_parts),
        "Album" | "Single" | "EP" => {
            parse_album_result(card, Some("Top result".to_owned()), title, &subtitle_parts)
        }
        "Playlist" => parse_playlist_result(
            card,
            Some("Top result".to_owned()),
            title,
            &subtitle_parts,
            true,
        ),
        other => Err(Error::Parse(format!(
            "unsupported top result type in default mixed fixture: {other}"
        ))),
    }
}

fn parse_shelf_item(item: &Value, category: Option<String>) -> Result<SearchResult, Error> {
    let renderer = required_value_at(item, "/musicResponsiveListItemRenderer")?;
    let title = required_runs_text_at(
        renderer,
        "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs",
    )?;
    let metadata_runs = required_array_at(
        renderer,
        "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs",
    )?;
    let metadata_parts = non_separator_runs(metadata_runs);
    let kind = metadata_parts
        .first()
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| {
            Error::Parse("search response missing shelf result type label".to_owned())
        })?;

    match kind.as_str() {
        "Album" | "Single" | "EP" => parse_album_result(renderer, category, title, &metadata_parts),
        "Song" => parse_song_result(renderer, category, title, &metadata_parts),
        "Video" => parse_video_result(renderer, category, title, &metadata_parts),
        "Artist" => parse_artist_result(renderer, category, title, &metadata_parts, false),
        "Profile" => parse_profile_result(renderer, category, title, &metadata_parts),
        "Playlist" => parse_playlist_result(renderer, category, title, &metadata_parts, true),
        "Episode" => parse_episode_result(renderer, category, title, &metadata_parts),
        "Podcast" => parse_podcast_result(renderer, category, title),
        other => Err(Error::Parse(format!(
            "unsupported shelf result type in default mixed fixture: {other}"
        ))),
    }
}

fn parse_filtered_shelf_item(
    item: &Value,
    category: Option<String>,
    filter: SearchFilter,
) -> Result<SearchResult, Error> {
    let renderer = required_value_at(item, "/musicResponsiveListItemRenderer")?;
    let title = required_runs_text_at(
        renderer,
        "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs",
    )?;
    let metadata_runs = required_array_at(
        renderer,
        "/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs",
    )?;
    let metadata_parts = non_separator_runs(metadata_runs);

    match filter {
        SearchFilter::Songs => parse_song_result(renderer, category, title, &metadata_parts),
        SearchFilter::Videos => parse_video_result(renderer, category, title, &metadata_parts),
        SearchFilter::Albums => parse_album_result(renderer, category, title, &metadata_parts),
        SearchFilter::Artists => {
            parse_artist_result(renderer, category, title, &metadata_parts, true)
        }
        SearchFilter::Playlists => {
            parse_playlist_result(renderer, category, title, &metadata_parts, false)
        }
    }
}

fn parse_album_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    let artist_run = metadata_parts
        .get(1)
        .ok_or_else(|| Error::Parse("search response missing album artist".to_owned()))?;
    let year = metadata_parts
        .get(2)
        .map(|run| required_text(run, "/text"))
        .transpose()?;

    Ok(SearchResult::Album(AlbumResult {
        category,
        result_type: SearchResultType::Album,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        playlist_id: optional_text_at(
            renderer,
            "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId",
        ),
        title,
        type_label: required_text(metadata_parts[0], "/text")?,
        year,
        duration: None,
        is_explicit: has_explicit_badge(renderer),
        artists: vec![parse_artist_ref(artist_run)?],
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

fn parse_song_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    let metadata = parse_media_metadata(metadata_parts);

    Ok(SearchResult::Song(SongResult {
        category,
        result_type: SearchResultType::Song,
        video_id: required_video_id(renderer)?,
        title,
        artists: metadata.artists,
        album: metadata.album,
        duration: metadata.duration,
        thumbnails: parse_thumbnails(renderer)?,
        is_explicit: has_explicit_badge(renderer),
    }))
}

fn parse_video_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    let metadata = parse_media_metadata(metadata_parts);

    Ok(SearchResult::Video(VideoResult {
        category,
        result_type: SearchResultType::Video,
        title,
        video_id: required_video_id(renderer)?,
        video_type: optional_video_type(renderer),
        artists: metadata.artists,
        thumbnails: parse_thumbnails(renderer)?,
        duration: metadata.duration,
        views: metadata.views,
        date: None,
        podcast: None,
        live: None,
    }))
}

fn parse_artist_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
    preserve_subscribers: bool,
) -> Result<SearchResult, Error> {
    let browse_id = required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?;
    let subscribers = preserve_subscribers
        .then(|| {
            metadata_parts
                .get(1)
                .map(|run| required_text(run, "/text"))
                .transpose()
        })
        .transpose()?
        .flatten();
    let menu_playlist_ids = renderer
        .pointer("/menu/menuRenderer/items")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.pointer(
                        "/menuNavigationItemRenderer/navigationEndpoint/watchPlaylistEndpoint/playlistId",
                    )
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(SearchResult::Artist(ArtistResult {
        category,
        result_type: SearchResultType::Artist,
        artist: Some(title),
        artists: Vec::new(),
        subscribers,
        browse_id: Some(browse_id),
        radio_id: menu_playlist_ids.get(1).cloned(),
        shuffle_id: menu_playlist_ids.first().cloned(),
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

fn parse_profile_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    let handle = metadata_parts
        .get(1)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing profile handle".to_owned()))?;

    Ok(SearchResult::Profile(ProfileResult {
        category,
        result_type: SearchResultType::Profile,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        name: title,
        handle,
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

fn parse_playlist_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
    has_type_label: bool,
) -> Result<SearchResult, Error> {
    let author_index = usize::from(has_type_label);
    let count_index = author_index + 1;
    let author = metadata_parts
        .get(author_index)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing playlist author".to_owned()))?;
    let item_count = metadata_parts
        .get(count_index)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .and_then(|value| {
            let lower = value.to_ascii_lowercase();
            if lower.contains("view") {
                return None;
            }
            if has_type_label {
                value.contains("song").then(|| first_token(value)).flatten()
            } else {
                first_token(value)
            }
        });

    Ok(SearchResult::Playlist(PlaylistResult {
        category,
        result_type: SearchResultType::Playlist,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        title,
        author: Some(author),
        item_count,
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

fn parse_episode_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    metadata_parts: &[&Value],
) -> Result<SearchResult, Error> {
    let date = metadata_parts
        .get(1)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing episode date".to_owned()))?;
    let podcast_run = metadata_parts
        .get(2)
        .ok_or_else(|| Error::Parse("search response missing episode podcast".to_owned()))?;

    Ok(SearchResult::Episode(VideoResult {
        category,
        result_type: SearchResultType::Episode,
        title,
        video_id: required_text(
            renderer,
            "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId",
        )?,
        video_type: Some(required_text(
            renderer,
            "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType",
        )?),
        artists: Vec::new(),
        thumbnails: parse_thumbnails(renderer)?,
        duration: None,
        views: None,
        date: Some(date),
        podcast: Some(parse_album_ref(podcast_run)?),
        live: Some(renderer.pointer("/badges/0/liveBadgeRenderer").is_some()),
    }))
}

fn parse_podcast_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
) -> Result<SearchResult, Error> {
    Ok(SearchResult::Podcast(PlaylistResult {
        category,
        result_type: SearchResultType::Podcast,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        title,
        author: None,
        item_count: None,
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

struct ParsedMediaMetadata {
    artists: Vec<ArtistRef>,
    album: Option<AlbumRef>,
    duration: Option<String>,
    views: Option<String>,
}

fn parse_media_metadata(metadata_parts: &[&Value]) -> ParsedMediaMetadata {
    let mut parsed = ParsedMediaMetadata {
        artists: Vec::new(),
        album: None,
        duration: None,
        views: None,
    };

    for part in metadata_parts {
        if let Some(browse_id) = part
            .pointer("/navigationEndpoint/browseEndpoint/browseId")
            .and_then(Value::as_str)
        {
            let name = part
                .pointer("/text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if browse_id.starts_with("MPRE") || browse_id.contains("release_detail") {
                if parsed.album.is_none() {
                    parsed.album = Some(AlbumRef {
                        id: browse_id.to_owned(),
                        name,
                    });
                }
            } else {
                parsed.artists.push(ArtistRef {
                    id: browse_id.to_owned(),
                    name,
                });
            }
            continue;
        }

        let Some(text) = part.pointer("/text").and_then(Value::as_str) else {
            continue;
        };

        if parsed.duration.is_none() && looks_like_duration(text) {
            parsed.duration = Some(text.to_owned());
        } else if parsed.views.is_none() && text.to_ascii_lowercase().contains("view") {
            parsed.views = Some(text.to_owned());
        }
    }

    parsed
}

fn parse_artist_ref(run: &Value) -> Result<ArtistRef, Error> {
    Ok(ArtistRef {
        id: required_text(run, "/navigationEndpoint/browseEndpoint/browseId")?,
        name: required_text(run, "/text")?,
    })
}

fn parse_album_ref(run: &Value) -> Result<AlbumRef, Error> {
    Ok(AlbumRef {
        id: required_text(run, "/navigationEndpoint/browseEndpoint/browseId")?,
        name: required_text(run, "/text")?,
    })
}

fn parse_thumbnails(value: &Value) -> Result<Vec<Thumbnail>, Error> {
    required_array_at(
        value,
        "/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails",
    )?
    .iter()
    .map(|thumbnail| {
        Ok(Thumbnail {
            height: required_u32(thumbnail, "/height")?,
            url: required_text(thumbnail, "/url")?,
            width: required_u32(thumbnail, "/width")?,
        })
    })
    .collect()
}

fn has_explicit_badge(value: &Value) -> bool {
    value
        .pointer("/badges")
        .and_then(Value::as_array)
        .is_some_and(|badges| {
            badges.iter().any(|badge| {
                badge
                    .pointer("/musicInlineBadgeRenderer/icon/iconType")
                    .and_then(Value::as_str)
                    == Some("MUSIC_EXPLICIT_BADGE")
            })
        })
}

fn first_token(value: String) -> Option<String> {
    value.split_whitespace().next().map(str::to_owned)
}

fn non_separator_runs(runs: &[Value]) -> Vec<&Value> {
    runs.iter()
        .filter(|run| {
            run.pointer("/text")
                .and_then(Value::as_str)
                .map(|text| text != " • " && !text.is_empty())
                .unwrap_or(false)
        })
        .collect()
}

fn optional_text_at(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn required_video_id(value: &Value) -> Result<String, Error> {
    optional_text_at(
        value,
        "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/videoId",
    )
    .or_else(|| optional_text_at(value, "/onTap/watchEndpoint/videoId"))
    .ok_or_else(|| Error::Parse("search response missing song/video id".to_owned()))
}

fn optional_video_type(value: &Value) -> Option<String> {
    optional_text_at(
        value,
        "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType",
    )
    .or_else(|| {
        optional_text_at(
            value,
            "/onTap/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType",
        )
    })
}

fn looks_like_duration(value: &str) -> bool {
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

fn optional_runs_text_at(value: &Value, pointer: &str) -> Option<String> {
    let runs = value.pointer(pointer)?.as_array()?;
    let mut text = String::new();
    for run in runs {
        text.push_str(run.pointer("/text").and_then(Value::as_str)?);
    }

    Some(text)
}

fn required_runs_text_at(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_runs_text_at(value, pointer)
        .ok_or_else(|| Error::Parse(format!("search response missing {pointer}")))
}

fn required_array_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], Error> {
    required_value_at(value, pointer)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(format!("search response missing {pointer}")))
}

fn required_text(value: &Value, pointer: &str) -> Result<String, Error> {
    required_value_at(value, pointer)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| Error::Parse(format!("search response missing {pointer}")))
}

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = required_value_at(value, pointer)?
        .as_u64()
        .ok_or_else(|| Error::Parse(format!("search response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("search response missing {pointer}")))
}

fn required_value_at<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value, Error> {
    value
        .pointer(pointer)
        .ok_or_else(|| Error::Parse(format!("search response missing {pointer}")))
}

#[cfg(test)]
mod tests {
    use super::{parse_search_continuation_response, parse_search_response};
    use crate::{ContinuationToken, SearchFilter, SearchResult};
    use serde_json::{Value, json};

    fn parse_raw_fixture(raw_fixture: &str, filter: Option<SearchFilter>) -> Vec<SearchResult> {
        let response: Value = serde_json::from_str(raw_fixture).unwrap();
        parse_search_response(&response, filter).unwrap().items
    }

    fn parse_fixture(fixture_name: &str, filter: Option<SearchFilter>) -> Vec<SearchResult> {
        let raw_fixture = match fixture_name {
            "songs_authenticated" => {
                include_str!("../../tests/fixtures/search/raw/songs_authenticated.json")
            }
            "videos_authenticated" => {
                include_str!("../../tests/fixtures/search/raw/videos_authenticated.json")
            }
            other => panic!("unknown fixture {other}"),
        };

        parse_raw_fixture(raw_fixture, filter)
    }

    fn expected_fixture(expected_fixture: &str) -> Value {
        serde_json::from_str(expected_fixture).unwrap()
    }

    fn parse_default_mixed() -> Vec<SearchResult> {
        parse_raw_fixture(
            include_str!("../../tests/fixtures/search/raw/default_mixed.json"),
            None,
        )
    }

    fn parse_albums() -> Vec<SearchResult> {
        parse_raw_fixture(
            include_str!("../../tests/fixtures/search/raw/albums.json"),
            Some(SearchFilter::Albums),
        )
    }

    fn parse_artists() -> Vec<SearchResult> {
        parse_raw_fixture(
            include_str!("../../tests/fixtures/search/raw/artists.json"),
            Some(SearchFilter::Artists),
        )
    }

    fn parse_playlists() -> Vec<SearchResult> {
        parse_raw_fixture(
            include_str!("../../tests/fixtures/search/raw/playlists.json"),
            Some(SearchFilter::Playlists),
        )
    }

    fn filtered_songs_continuation_response() -> Value {
        let response: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/search/raw/songs_authenticated.json"
        ))
        .unwrap();
        let contents = response["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"]["contents"][0]
            ["musicShelfRenderer"]["contents"]
            .clone();

        json!({
            "continuationContents": {
                "musicShelfContinuation": {
                    "contents": contents,
                    "continuations": [{
                        "nextContinuationData": {
                            "continuation": "songs-token-2"
                        }
                    }]
                }
            }
        })
    }

    fn filtered_artist_continuation_response_without_subscribers() -> Value {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/search/raw/artists.json"))
                .unwrap();
        let mut contents = response["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"]["contents"][0]
            ["musicShelfRenderer"]["contents"]
            .clone();
        let artist_item = &mut contents[3]["musicResponsiveListItemRenderer"];
        artist_item["flexColumns"][1]["musicResponsiveListItemFlexColumnRenderer"]["text"]["runs"] =
            json!([{ "text": "Armin van Buuren ASOT Radio" }]);

        json!({
            "continuationContents": {
                "musicShelfContinuation": {
                    "contents": [contents[3].clone()],
                    "continuations": [{
                        "nextContinuationData": {
                            "continuation": "artists-token-1"
                        }
                    }]
                }
            }
        })
    }

    fn default_mixed_continuation_response() -> Value {
        let response: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/search/raw/default_mixed.json"
        ))
        .unwrap();
        let contents = response["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"]["contents"][1]
            ["musicShelfRenderer"]["contents"]
            .clone();

        json!({
            "continuationContents": {
                "musicShelfContinuation": {
                    "contents": contents,
                    "continuations": [{
                        "nextContinuationData": {
                            "continuation": "search-token-2"
                        }
                    }]
                }
            }
        })
    }

    fn parse_inline_default_mixed(sections: Vec<Value>) -> Vec<SearchResult> {
        parse_search_response(
            &json!({
                "contents": {
                    "tabbedSearchResultsRenderer": {
                        "tabs": [{
                            "tabRenderer": {
                                "content": {
                                    "sectionListRenderer": {
                                        "contents": sections
                                    }
                                }
                            }
                        }]
                    }
                }
            }),
            None,
        )
        .unwrap()
        .items
    }

    fn parse_inline_page(
        sections: Vec<Value>,
        filter: Option<SearchFilter>,
    ) -> crate::Page<SearchResult> {
        parse_search_response(
            &json!({
                "contents": {
                    "tabbedSearchResultsRenderer": {
                        "tabs": [{
                            "tabRenderer": {
                                "content": {
                                    "sectionListRenderer": {
                                        "contents": sections
                                    }
                                }
                            }
                        }]
                    }
                }
            }),
            filter,
        )
        .unwrap()
    }

    fn expected_default_mixed() -> Value {
        expected_fixture(include_str!(
            "../../tests/fixtures/search/expected/default_mixed.json"
        ))
    }

    fn expected_albums() -> Value {
        expected_fixture(include_str!(
            "../../tests/fixtures/search/expected/albums.json"
        ))
    }

    fn expected_artists() -> Value {
        expected_fixture(include_str!(
            "../../tests/fixtures/search/expected/artists.json"
        ))
    }

    fn expected_playlists() -> Value {
        expected_fixture(include_str!(
            "../../tests/fixtures/search/expected/playlists.json"
        ))
    }

    #[test]
    fn default_mixed_fixture_matches_expected_snapshot() {
        let parsed = parse_default_mixed();

        assert_eq!(
            serde_json::to_value(parsed).unwrap(),
            expected_default_mixed()
        );
    }

    #[test]
    fn default_mixed_fixture_reports_continuation() {
        let mut response: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/search/raw/default_mixed.json"
        ))
        .unwrap();
        response["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]["tabRenderer"]["content"]
            ["sectionListRenderer"]["continuations"] = json!([{
            "nextContinuationData": {
                "continuation": "search-token-1"
            }
        }]);

        let parsed = parse_search_response(&response, None).unwrap();

        assert_eq!(
            parsed.continuation,
            Some(ContinuationToken::new("search-token-1").unwrap())
        );
    }

    #[test]
    fn default_mixed_fixture_preserves_critical_invariants() {
        let parsed = parse_default_mixed();

        assert!(matches!(
            &parsed[5],
            SearchResult::Playlist(result)
                if result.title == "Best Of Daft Punk"
                    && result.author.as_deref() == Some("misterepicpants")
                    && result.item_count.is_none()
        ));
        assert!(matches!(
            &parsed[14],
            SearchResult::Playlist(result)
                if result.title == "Presenting Daft Punk"
                    && result.author.as_deref() == Some("YouTube Music")
                    && result.item_count.as_deref() == Some("35")
        ));
        assert!(matches!(
            &parsed[16],
            SearchResult::Profile(result)
                if result.name == "daft punk"
                    && result.handle == "@daftpunk7519"
                    && result.browse_id == "UCqYzmWVTHBszT8J3VGC1zIw"
        ));
        assert!(matches!(
            &parsed[10],
            SearchResult::Episode(result)
                if result
                    .podcast
                    .as_ref()
                    .map(|podcast| podcast.name.as_str())
                    == Some("🌟 Funkzone Sound – Future Funk Podcast 🌟")
        ));
        assert!(matches!(
            &parsed[21],
            SearchResult::Podcast(result) if result.title == "off Track Podcast Season 2"
        ));
    }

    #[test]
    fn albums_fixture_matches_expected_snapshot() {
        let parsed = parse_albums();

        assert_eq!(serde_json::to_value(parsed).unwrap(), expected_albums());
    }

    #[test]
    fn artists_fixture_matches_expected_snapshot() {
        let parsed = parse_artists();

        assert_eq!(serde_json::to_value(parsed).unwrap(), expected_artists());
    }

    #[test]
    fn playlists_fixture_matches_expected_snapshot() {
        let parsed = parse_playlists();

        assert_eq!(serde_json::to_value(parsed).unwrap(), expected_playlists());
    }

    #[test]
    fn filtered_fixtures_preserve_critical_invariants() {
        let albums = parse_albums();
        let artists = parse_artists();
        let playlists = parse_playlists();

        assert!(matches!(
            &albums[0],
            SearchResult::Album(result)
                if result.title == "Relapse"
                    && result.is_explicit
                    && result.year.as_deref() == Some("2009")
                    && result.artists[0].name == "Eminem"
        ));
        assert!(matches!(
            &artists[3],
            SearchResult::Artist(result)
                if result.artist.as_deref() == Some("Armin van Buuren ASOT Radio")
                    && result.subscribers.as_deref() == Some("5.74K subscribers")
                    && result.shuffle_id.is_none()
                    && result.radio_id.is_none()
        ));
        assert!(matches!(
            &playlists[0],
            SearchResult::Playlist(result)
                if result.title == "best 100 classical music"
                    && result.author.as_deref() == Some("Adam")
                    && result.item_count.is_none()
        ));
    }

    #[test]
    fn filtered_songs_continuation_is_found_on_any_shelf() {
        let response: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/search/raw/songs_authenticated.json"
        ))
        .unwrap();
        let mut sections = response["contents"]["tabbedSearchResultsRenderer"]["tabs"][0]
            ["tabRenderer"]["content"]["sectionListRenderer"]["contents"]
            .as_array()
            .unwrap()
            .clone();
        let second_section = sections[0].clone();
        sections.push(second_section);
        sections[1]["musicShelfRenderer"]["continuations"] = json!([{
            "nextContinuationData": {
                "continuation": "songs-token-1"
            }
        }]);

        let parsed = parse_inline_page(sections, Some(SearchFilter::Songs));

        assert!(
            parsed
                .items
                .iter()
                .all(|result| matches!(result, SearchResult::Song(_)))
        );
        assert_eq!(
            parsed.continuation,
            Some(ContinuationToken::new("songs-token-1").unwrap())
        );
    }

    #[test]
    fn filtered_songs_continuation_response_parses_items_and_token() {
        let parsed =
            parse_search_continuation_response(&filtered_songs_continuation_response()).unwrap();

        assert!(
            parsed
                .items
                .iter()
                .all(|result| matches!(result, SearchResult::Song(_)))
        );
        assert_eq!(
            parsed.continuation,
            Some(ContinuationToken::new("songs-token-2").unwrap())
        );
    }

    #[test]
    fn filtered_artist_continuation_without_subscribers_remains_artist() {
        let parsed = parse_search_continuation_response(
            &filtered_artist_continuation_response_without_subscribers(),
        )
        .unwrap();

        assert!(matches!(
            &parsed.items[0],
            SearchResult::Artist(result)
                if result.artist.as_deref() == Some("Armin van Buuren ASOT Radio")
                    && result.subscribers.is_none()
        ));
        assert_eq!(
            parsed.continuation,
            Some(ContinuationToken::new("artists-token-1").unwrap())
        );
    }

    #[test]
    fn default_mixed_continuation_response_parses_items_and_token() {
        let parsed =
            parse_search_continuation_response(&default_mixed_continuation_response()).unwrap();

        assert!(matches!(
            &parsed.items[0],
            SearchResult::Album(result) if result.title == "Random Access Memories"
        ));
        assert!(matches!(
            &parsed.items[1],
            SearchResult::Album(result) if result.title == "Discovery"
        ));
        assert_eq!(
            parsed.continuation,
            Some(ContinuationToken::new("search-token-2").unwrap())
        );
    }

    #[test]
    fn authenticated_songs_fixture_parses_non_empty_song_results() {
        let results = parse_fixture("songs_authenticated", Some(SearchFilter::Songs));
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .all(|result| matches!(result, SearchResult::Song(_)))
        );

        let SearchResult::Song(first_song) = &results[0] else {
            panic!("expected first authenticated songs result to be a song");
        };
        assert_eq!(first_song.artists[0].name, "ABBA");
        assert_eq!(
            first_song.album.as_ref().map(|album| album.name.as_str()),
            Some("The Visitors")
        );
        assert_eq!(first_song.duration.as_deref(), Some("3:56"));
    }

    #[test]
    fn authenticated_videos_fixture_parses_non_empty_video_results() {
        let results = parse_fixture("videos_authenticated", Some(SearchFilter::Videos));
        assert!(!results.is_empty());
        assert!(
            results
                .iter()
                .all(|result| matches!(result, SearchResult::Video(_)))
        );

        let SearchResult::Video(first_video) = &results[0] else {
            panic!("expected first authenticated videos result to be a video");
        };
        assert_eq!(first_video.artists[0].name, "Laugh Zone");
        assert_eq!(first_video.views.as_deref(), Some("233K views"));
        assert_eq!(first_video.duration.as_deref(), Some("1:09:21"));
    }

    #[test]
    fn default_mixed_parses_song_top_result_cards() {
        let parsed = parse_inline_default_mixed(vec![json!({
            "musicCardShelfRenderer": {
                "title": { "runs": [{ "text": "Wonderwall" }] },
                "subtitle": {
                    "runs": [
                        { "text": "Song" },
                        { "text": " • " },
                        {
                            "text": "Oasis",
                            "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                        },
                        { "text": " • " },
                        {
                            "text": "(What's the Story) Morning Glory?",
                            "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_wvE8SfA1VxJ" } }
                        },
                        { "text": " • " },
                        { "text": "4:18" }
                    ]
                },
                "onTap": {
                    "watchEndpoint": {
                        "videoId": "song-top-result-id",
                        "watchEndpointMusicSupportedConfigs": {
                            "watchEndpointMusicConfig": {
                                "musicVideoType": "MUSIC_VIDEO_TYPE_ATV"
                            }
                        }
                    }
                },
                "thumbnail": {
                    "musicThumbnailRenderer": {
                        "thumbnail": {
                            "thumbnails": [{ "url": "https://example.com/song-top.jpg", "width": 60, "height": 60 }]
                        }
                    }
                }
            }
        })]);

        assert!(matches!(
            &parsed[0],
            SearchResult::Song(result)
                if result.category.as_deref() == Some("Top result")
                    && result.title == "Wonderwall"
                    && result.video_id == "song-top-result-id"
                    && result.duration.as_deref() == Some("4:18")
                    && result.album.as_ref().map(|album| album.name.as_str())
                        == Some("(What's the Story) Morning Glory?")
                    && result.artists.iter().map(|artist| artist.name.as_str()).collect::<Vec<_>>()
                        == vec!["Oasis"]
        ));
    }

    #[test]
    fn default_mixed_parses_video_top_result_cards() {
        let parsed = parse_inline_default_mixed(vec![json!({
            "musicCardShelfRenderer": {
                "title": { "runs": [{ "text": "Live Forever" }] },
                "subtitle": {
                    "runs": [
                        { "text": "Video" },
                        { "text": " • " },
                        {
                            "text": "Oasis",
                            "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                        },
                        { "text": " • " },
                        { "text": "4:37" }
                    ]
                },
                "onTap": {
                    "watchEndpoint": {
                        "videoId": "video-top-result-id",
                        "watchEndpointMusicSupportedConfigs": {
                            "watchEndpointMusicConfig": {
                                "musicVideoType": "MUSIC_VIDEO_TYPE_OMV"
                            }
                        }
                    }
                },
                "thumbnail": {
                    "musicThumbnailRenderer": {
                        "thumbnail": {
                            "thumbnails": [{ "url": "https://example.com/video-top.jpg", "width": 60, "height": 60 }]
                        }
                    }
                }
            }
        })]);

        assert!(matches!(
            &parsed[0],
            SearchResult::Video(result)
                if result.category.as_deref() == Some("Top result")
                    && result.title == "Live Forever"
                    && result.video_id == "video-top-result-id"
                    && result.video_type.as_deref() == Some("MUSIC_VIDEO_TYPE_OMV")
                    && result.duration.as_deref() == Some("4:37")
                    && result.artists.iter().map(|artist| artist.name.as_str()).collect::<Vec<_>>()
                        == vec!["Oasis"]
        ));
    }

    #[test]
    fn default_mixed_parses_album_top_result_cards() {
        let parsed = parse_inline_default_mixed(vec![json!({
            "musicCardShelfRenderer": {
                "title": { "runs": [{ "text": "Definitely Maybe" }] },
                "subtitle": {
                    "runs": [
                        { "text": "Album" },
                        { "text": " • " },
                        {
                            "text": "Oasis",
                            "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                        },
                        { "text": " • " },
                        { "text": "1994" }
                    ]
                },
                "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_album_top" } },
                "thumbnail": {
                    "musicThumbnailRenderer": {
                        "thumbnail": {
                            "thumbnails": [{ "url": "https://example.com/album-top.jpg", "width": 60, "height": 60 }]
                        }
                    }
                }
            }
        })]);

        assert!(matches!(
            &parsed[0],
            SearchResult::Album(result)
                if result.category.as_deref() == Some("Top result")
                    && result.title == "Definitely Maybe"
                    && result.browse_id == "MPREb_album_top"
                    && result.playlist_id.is_none()
                    && result.type_label == "Album"
                    && result.year.as_deref() == Some("1994")
                    && result.artists.iter().map(|artist| artist.name.as_str()).collect::<Vec<_>>()
                        == vec!["Oasis"]
        ));
    }

    #[test]
    fn default_mixed_parses_playlist_top_result_cards() {
        let parsed = parse_inline_default_mixed(vec![json!({
            "musicCardShelfRenderer": {
                "title": { "runs": [{ "text": "Best Of Oasis" }] },
                "subtitle": {
                    "runs": [
                        { "text": "Playlist" },
                        { "text": " • " },
                        { "text": "YouTube Music" },
                        { "text": " • " },
                        { "text": "35 songs" }
                    ]
                },
                "navigationEndpoint": { "browseEndpoint": { "browseId": "VLPL_top_playlist" } },
                "thumbnail": {
                    "musicThumbnailRenderer": {
                        "thumbnail": {
                            "thumbnails": [{ "url": "https://example.com/playlist-top.jpg", "width": 60, "height": 60 }]
                        }
                    }
                }
            }
        })]);

        assert!(matches!(
            &parsed[0],
            SearchResult::Playlist(result)
                if result.category.as_deref() == Some("Top result")
                    && result.title == "Best Of Oasis"
                    && result.browse_id == "VLPL_top_playlist"
                    && result.author.as_deref() == Some("YouTube Music")
                    && result.item_count.as_deref() == Some("35")
        ));
    }

    #[test]
    fn filtered_album_rows_tolerate_missing_year_and_playlist_id() {
        let parsed = parse_search_response(
            &json!({
                "contents": {
                    "tabbedSearchResultsRenderer": {
                        "tabs": [{
                            "tabRenderer": {
                                "content": {
                                    "sectionListRenderer": {
                                        "contents": [{
                                            "musicShelfRenderer": {
                                                "title": { "runs": [{ "text": "Albums" }] },
                                                "contents": [{
                                                    "musicResponsiveListItemRenderer": {
                                                        "flexColumns": [
                                                            {
                                                                "musicResponsiveListItemFlexColumnRenderer": {
                                                                    "text": { "runs": [{ "text": "Heathen Chemistry" }] }
                                                                }
                                                            },
                                                            {
                                                                "musicResponsiveListItemFlexColumnRenderer": {
                                                                    "text": {
                                                                        "runs": [
                                                                            { "text": "Album" },
                                                                            { "text": " • " },
                                                                            {
                                                                                "text": "Oasis",
                                                                                "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                                                                            }
                                                                        ]
                                                                    }
                                                                }
                                                            }
                                                        ],
                                                        "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_album_row" } },
                                                        "thumbnail": {
                                                            "musicThumbnailRenderer": {
                                                                "thumbnail": {
                                                                    "thumbnails": [{ "url": "https://example.com/album-row.jpg", "width": 60, "height": 60 }]
                                                                }
                                                            }
                                                        }
                                                    }
                                                }]
                                            }
                                        }]
                                    }
                                }
                            }
                        }]
                    }
                }
            }),
            Some(SearchFilter::Albums),
        )
        .unwrap();

        assert!(matches!(
            &parsed.items[0],
            SearchResult::Album(result)
                if result.category.as_deref() == Some("Albums")
                    && result.title == "Heathen Chemistry"
                    && result.browse_id == "MPREb_album_row"
                    && result.playlist_id.is_none()
                    && result.year.is_none()
                    && result.artists.iter().map(|artist| artist.name.as_str()).collect::<Vec<_>>()
                        == vec!["Oasis"]
        ));
    }

    #[test]
    fn default_mixed_parses_song_and_video_shelf_rows() {
        let parsed = parse_inline_default_mixed(vec![json!({
            "musicShelfRenderer": {
                "title": { "runs": [{ "text": "Songs" }] },
                "contents": [
                    {
                        "musicResponsiveListItemRenderer": {
                            "flexColumns": [
                                {
                                    "musicResponsiveListItemFlexColumnRenderer": {
                                        "text": { "runs": [{ "text": "Wonderwall" }] }
                                    }
                                },
                                {
                                    "musicResponsiveListItemFlexColumnRenderer": {
                                        "text": {
                                            "runs": [
                                                { "text": "Song" },
                                                { "text": " • " },
                                                {
                                                    "text": "Oasis",
                                                    "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                                                },
                                                { "text": " • " },
                                                {
                                                    "text": "(What's the Story) Morning Glory?",
                                                    "navigationEndpoint": { "browseEndpoint": { "browseId": "MPREb_wvE8SfA1VxJ" } }
                                                },
                                                { "text": " • " },
                                                { "text": "4:18" }
                                            ]
                                        }
                                    }
                                }
                            ],
                            "overlay": {
                                "musicItemThumbnailOverlayRenderer": {
                                    "content": {
                                        "musicPlayButtonRenderer": {
                                            "playNavigationEndpoint": {
                                                "watchEndpoint": {
                                                    "videoId": "song-shelf-id",
                                                    "watchEndpointMusicSupportedConfigs": {
                                                        "watchEndpointMusicConfig": {
                                                            "musicVideoType": "MUSIC_VIDEO_TYPE_ATV"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "thumbnail": {
                                "musicThumbnailRenderer": {
                                    "thumbnail": {
                                        "thumbnails": [{ "url": "https://example.com/song-shelf.jpg", "width": 60, "height": 60 }]
                                    }
                                }
                            }
                        }
                    },
                    {
                        "musicResponsiveListItemRenderer": {
                            "flexColumns": [
                                {
                                    "musicResponsiveListItemFlexColumnRenderer": {
                                        "text": { "runs": [{ "text": "Live Forever" }] }
                                    }
                                },
                                {
                                    "musicResponsiveListItemFlexColumnRenderer": {
                                        "text": {
                                            "runs": [
                                                { "text": "Video" },
                                                { "text": " • " },
                                                {
                                                    "text": "Oasis",
                                                    "navigationEndpoint": { "browseEndpoint": { "browseId": "UCmMUZbaYdNH0bEd1PAlAqsA" } }
                                                },
                                                { "text": " • " },
                                                { "text": "4:37" }
                                            ]
                                        }
                                    }
                                }
                            ],
                            "overlay": {
                                "musicItemThumbnailOverlayRenderer": {
                                    "content": {
                                        "musicPlayButtonRenderer": {
                                            "playNavigationEndpoint": {
                                                "watchEndpoint": {
                                                    "videoId": "video-shelf-id",
                                                    "watchEndpointMusicSupportedConfigs": {
                                                        "watchEndpointMusicConfig": {
                                                            "musicVideoType": "MUSIC_VIDEO_TYPE_OMV"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                            "thumbnail": {
                                "musicThumbnailRenderer": {
                                    "thumbnail": {
                                        "thumbnails": [{ "url": "https://example.com/video-shelf.jpg", "width": 60, "height": 60 }]
                                    }
                                }
                            }
                        }
                    }
                ]
            }
        })]);

        assert!(matches!(
            &parsed[0],
            SearchResult::Song(result)
                if result.category.as_deref() == Some("Songs")
                    && result.video_id == "song-shelf-id"
                    && result.duration.as_deref() == Some("4:18")
                    && result.album.as_ref().map(|album| album.name.as_str())
                        == Some("(What's the Story) Morning Glory?")
        ));
        assert!(matches!(
            &parsed[1],
            SearchResult::Video(result)
                if result.category.as_deref() == Some("Songs")
                    && result.video_id == "video-shelf-id"
                    && result.video_type.as_deref() == Some("MUSIC_VIDEO_TYPE_OMV")
                    && result.duration.as_deref() == Some("4:37")
        ));
    }
}
