use serde_json::Value;

use crate::{
    Error, SearchFilter,
    model::{
        common::{AlbumRef, ArtistRef, Thumbnail},
        search::{
            AlbumResult, ArtistResult, PlaylistResult, ProfileResult, SearchResult,
            SearchResultType, VideoResult,
        },
    },
};

pub fn parse_search_response(
    response: &Value,
    filter: Option<SearchFilter>,
) -> Result<Vec<SearchResult>, Error> {
    if filter.is_some() {
        return Err(Error::UnsupportedFeature(
            "search parser currently supports only default mixed responses".to_owned(),
        ));
    }

    let sections = required_array_at(
        response,
        "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
    )?;

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

fn parse_top_result(card: &Value) -> Result<SearchResult, Error> {
    let title_runs = required_array_at(card, "/title/runs")?;
    let artist_run = title_runs
        .first()
        .ok_or_else(|| Error::Parse("search response missing /title/runs/0".to_owned()))?;
    let subtitle_runs = required_array_at(card, "/subtitle/runs")?;
    let subtitle_parts = non_separator_runs(subtitle_runs);
    let result_kind = subtitle_parts
        .first()
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing top result type label".to_owned()))?;

    if result_kind != "Artist" {
        return Err(Error::Parse(format!(
            "unsupported top result type in default mixed fixture: {result_kind}"
        )));
    }

    let subscribers = subtitle_parts
        .get(1)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .and_then(|value| value.split_whitespace().next().map(str::to_owned))
        .ok_or_else(|| Error::Parse("search response missing top result subscribers".to_owned()))?;

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
        "Artist" => parse_artist_result(renderer, category, title, true),
        "Profile" => parse_profile_result(renderer, category, title, &metadata_parts),
        "Playlist" => parse_playlist_result(renderer, category, title, &metadata_parts),
        "Episode" => parse_episode_result(renderer, category, title, &metadata_parts),
        "Podcast" => parse_podcast_result(renderer, category, title),
        other => Err(Error::Parse(format!(
            "unsupported shelf result type in default mixed fixture: {other}"
        ))),
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
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing album year".to_owned()))?;

    Ok(SearchResult::Album(AlbumResult {
        category,
        result_type: SearchResultType::Album,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        playlist_id: required_text(
            renderer,
            "/overlay/musicItemThumbnailOverlayRenderer/content/musicPlayButtonRenderer/playNavigationEndpoint/watchPlaylistEndpoint/playlistId",
        )?,
        title,
        type_label: required_text(metadata_parts[0], "/text")?,
        year: Some(year),
        duration: None,
        is_explicit: false,
        artists: vec![parse_artist_ref(artist_run)?],
        thumbnails: parse_thumbnails(renderer)?,
    }))
}

fn parse_artist_result(
    renderer: &Value,
    category: Option<String>,
    title: String,
    include_radio_data: bool,
) -> Result<SearchResult, Error> {
    let browse_id = required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?;

    let (shuffle_id, radio_id) = if include_radio_data {
        (
            Some(required_text(
                renderer,
                "/menu/menuRenderer/items/0/menuNavigationItemRenderer/navigationEndpoint/watchPlaylistEndpoint/playlistId",
            )?),
            Some(required_text(
                renderer,
                "/menu/menuRenderer/items/1/menuNavigationItemRenderer/navigationEndpoint/watchPlaylistEndpoint/playlistId",
            )?),
        )
    } else {
        (None, None)
    };

    Ok(SearchResult::Artist(ArtistResult {
        category,
        result_type: SearchResultType::Artist,
        artist: Some(title),
        artists: Vec::new(),
        subscribers: None,
        browse_id: Some(browse_id),
        radio_id,
        shuffle_id,
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
) -> Result<SearchResult, Error> {
    let author = metadata_parts
        .get(1)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .ok_or_else(|| Error::Parse("search response missing playlist author".to_owned()))?;
    let item_count = metadata_parts
        .get(2)
        .map(|run| required_text(run, "/text"))
        .transpose()?
        .and_then(|value| {
            value
                .contains("song")
                .then(|| value.split_whitespace().next().map(str::to_owned))
                .flatten()
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

    Ok(SearchResult::Video(VideoResult {
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
    Ok(SearchResult::Playlist(PlaylistResult {
        category,
        result_type: SearchResultType::Podcast,
        browse_id: required_text(renderer, "/navigationEndpoint/browseEndpoint/browseId")?,
        title,
        author: None,
        item_count: None,
        thumbnails: parse_thumbnails(renderer)?,
    }))
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
