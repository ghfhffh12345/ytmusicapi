use serde_json::Value;

use crate::{
    AlbumRef, ArtistRef, ContinuationToken, Error, LibraryLikeStatus, Page, Thumbnail, WatchTrack,
};

pub(crate) fn parse_watch_playlist_response(response: &Value) -> Result<Page<WatchTrack>, Error> {
    let renderer = response
        .pointer("/contents/singleColumnMusicWatchNextResultsRenderer/tabbedRenderer/watchNextTabbedResultsRenderer/tabs/0/tabRenderer/content/musicQueueRenderer/content/playlistPanelRenderer")
        .ok_or_else(|| Error::Parse("watch response missing playlistPanelRenderer".to_owned()))?;

    Ok(Page {
        items: parse_watch_tracks(required_array(renderer, "/contents")?)?,
        continuation: extract_continuation(renderer)?,
    })
}

pub(crate) fn parse_watch_playlist_continuation(
    response: &Value,
) -> Result<Page<WatchTrack>, Error> {
    let renderer = response
        .pointer("/continuationContents/playlistPanelContinuation")
        .ok_or_else(|| {
            Error::Parse("watch continuation missing playlistPanelContinuation".to_owned())
        })?;

    Ok(Page {
        items: parse_watch_tracks(required_array(renderer, "/contents")?)?,
        continuation: extract_continuation(renderer)?,
    })
}

fn parse_watch_tracks(items: &[Value]) -> Result<Vec<WatchTrack>, Error> {
    let mut parsed = Vec::new();

    for item in items {
        let (primary, counterpart) = if let Some(wrapper) =
            item.get("playlistPanelVideoWrapperRenderer")
        {
            (
                wrapper
                    .pointer("/primaryRenderer/playlistPanelVideoRenderer")
                    .ok_or_else(|| {
                        Error::Parse("watch wrapper missing primaryRenderer".to_owned())
                    })?,
                wrapper.pointer("/counterpart/0/counterpartRenderer/playlistPanelVideoRenderer"),
            )
        } else if let Some(renderer) = item.get("playlistPanelVideoRenderer") {
            (renderer, None)
        } else {
            continue;
        };

        if primary.get("unplayableText").is_some() {
            continue;
        }

        let mut track = parse_watch_track(primary)?;
        if let Some(counterpart) = counterpart {
            track.counterpart = Some(Box::new(parse_watch_track(counterpart)?));
        }
        parsed.push(track);
    }

    Ok(parsed)
}

fn parse_watch_track(renderer: &Value) -> Result<WatchTrack, Error> {
    let byline_runs = renderer
        .pointer("/longBylineText/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    Ok(WatchTrack {
        video_id: required_text(renderer, "/videoId")?,
        title: required_runs_text(renderer, "/title/runs")?,
        duration: optional_runs_text(renderer, "/lengthText/runs"),
        thumbnails: parse_thumbnails(renderer)?,
        artists: parse_artists(byline_runs),
        album: parse_album(byline_runs),
        like_status: parse_like_status(renderer),
        video_type: optional_text(
            renderer,
            "/navigationEndpoint/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType",
        ),
        year: parse_year(byline_runs),
        views: parse_views(byline_runs),
        is_in_library: None,
        counterpart: None,
    })
}

fn required_array<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(format!("watch response missing {pointer}")))
}

fn required_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("watch response missing {pointer}")))
}

fn required_runs_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_runs_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("watch response missing {pointer}")))
}

fn optional_text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn optional_runs_text(value: &Value, pointer: &str) -> Option<String> {
    let runs = value.pointer(pointer)?.as_array()?;
    let mut text = String::new();
    for run in runs {
        text.push_str(run.pointer("/text").and_then(Value::as_str)?);
    }

    Some(text)
}

fn parse_thumbnails(value: &Value) -> Result<Vec<Thumbnail>, Error> {
    required_array(
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

fn extract_continuation(value: &Value) -> Result<Option<ContinuationToken>, Error> {
    optional_text(value, "/continuations/0/nextContinuationData/continuation")
        .map(ContinuationToken::new)
        .transpose()
}

fn parse_artists(runs: &[Value]) -> Vec<ArtistRef> {
    runs.iter()
        .filter_map(|run| {
            let text = run.pointer("/text").and_then(Value::as_str)?;
            if text.is_empty()
                || text == " • "
                || looks_like_album_run(run)
                || looks_like_year(text)
                || looks_like_views(text)
            {
                return None;
            }

            Some(ArtistRef {
                id: optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
                    .unwrap_or_default(),
                name: text.to_owned(),
            })
        })
        .collect()
}

fn parse_album(runs: &[Value]) -> Option<AlbumRef> {
    runs.iter().find_map(|run| {
        let browse_id = optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")?;
        if !is_album_browse_id(&browse_id) {
            return None;
        }

        Some(AlbumRef {
            id: browse_id,
            name: run.pointer("/text").and_then(Value::as_str)?.to_owned(),
        })
    })
}

fn parse_year(runs: &[Value]) -> Option<String> {
    runs.iter().find_map(|run| {
        let text = run.pointer("/text").and_then(Value::as_str)?;
        looks_like_year(text).then(|| text.to_owned())
    })
}

fn parse_views(runs: &[Value]) -> Option<String> {
    runs.iter().find_map(|run| {
        let text = run.pointer("/text").and_then(Value::as_str)?;
        looks_like_views(text).then(|| text.to_owned())
    })
}

fn parse_like_status(renderer: &Value) -> Option<LibraryLikeStatus> {
    match optional_text(
        renderer,
        "/menu/menuRenderer/items/0/toggleMenuServiceItemRenderer/defaultServiceEndpoint/likeEndpoint/status",
    )
    .as_deref()
    {
        Some("LIKE") => Some(LibraryLikeStatus::Like),
        Some("INDIFFERENT") => Some(LibraryLikeStatus::Indifferent),
        Some("DISLIKE") => Some(LibraryLikeStatus::Dislike),
        _ => None,
    }
}

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("watch response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("watch response missing {pointer}")))
}

fn looks_like_album_run(run: &Value) -> bool {
    optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
        .is_some_and(|browse_id| is_album_browse_id(&browse_id))
}

fn is_album_browse_id(browse_id: &str) -> bool {
    browse_id.starts_with("MPRE") || browse_id.starts_with("FEmusic_")
}

fn looks_like_year(text: &str) -> bool {
    text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn looks_like_views(text: &str) -> bool {
    text.ends_with("views")
}

#[cfg(test)]
mod tests {
    use super::{parse_watch_playlist_continuation, parse_watch_playlist_response};
    use crate::{ContinuationToken, LibraryLikeStatus, WatchTrack};

    #[test]
    fn parse_watch_playlist_response_returns_items_counterpart_and_continuation() {
        let response: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/watch/raw/first_page.json"
        ))
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_response(&response).unwrap();

        assert_eq!(
            page.continuation,
            Some(ContinuationToken::new("watch-token-1").unwrap())
        );
        assert_eq!(page.items.len(), 2);
        assert_eq!(page.items[0].video_id, "video-1");
        assert_eq!(page.items[0].title, "Primary Song");
        assert_eq!(page.items[0].like_status, Some(LibraryLikeStatus::Like));
        assert_eq!(page.items[0].artists[0].id, "UCartist1");
        assert_eq!(page.items[0].album.as_ref().unwrap().id, "MPREalbum1");
        assert_eq!(
            page.items[0]
                .counterpart
                .as_ref()
                .map(|track| track.video_id.as_str()),
            Some("video-1-counterpart")
        );
        assert_eq!(
            page.items[0].counterpart.as_ref().unwrap().artists[0].id,
            ""
        );
    }

    #[test]
    fn parse_watch_playlist_continuation_returns_items_and_next_token() {
        let response: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/watch/raw/continuation.json"
        ))
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_continuation(&response).unwrap();

        assert_eq!(
            page.continuation,
            Some(ContinuationToken::new("watch-token-2").unwrap())
        );
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].video_id, "video-3");
        assert_eq!(page.items[0].artists[0].id, "UCartist3");
    }

    #[test]
    fn parse_watch_playlist_response_handles_radio_fixture() {
        let response: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/watch/raw/radio_first_page.json"
        ))
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_response(&response).unwrap();
        assert!(!page.items.is_empty());
    }

    #[test]
    fn parse_watch_playlist_response_handles_shuffle_fixture() {
        let response: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/watch/raw/shuffle_first_page.json"
        ))
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_response(&response).unwrap();
        assert!(!page.items.is_empty());
    }
}
