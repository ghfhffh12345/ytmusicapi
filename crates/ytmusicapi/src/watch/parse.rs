#![allow(dead_code)]

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
        is_in_library: parse_in_library(renderer),
        video_type: optional_text(
            renderer,
            "/navigationEndpoint/watchEndpoint/watchEndpointMusicSupportedConfigs/watchEndpointMusicConfig/musicVideoType",
        ),
        year: parse_year(byline_runs),
        views: parse_views(byline_runs),
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
    let thumbnails = required_array(
        value,
        "/thumbnail/musicThumbnailRenderer/thumbnail/thumbnails",
    )
    .or_else(|_| required_array(value, "/thumbnail/thumbnails"))?;

    thumbnails
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
            let trimmed = text.trim();
            if trimmed.is_empty()
                || trimmed == "•"
                || looks_like_year(trimmed)
                || looks_like_views(trimmed)
            {
                return None;
            }

            if let Some(browse_id) =
                optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")
            {
                if is_album_run(run, &browse_id) {
                    return None;
                }

                return Some(ArtistRef {
                    id: browse_id,
                    name: text.to_owned(),
                });
            }

            Some(ArtistRef {
                id: String::new(),
                name: text.to_owned(),
            })
        })
        .collect()
}

fn parse_album(runs: &[Value]) -> Option<AlbumRef> {
    runs.iter().find_map(|run| {
        let browse_id = optional_text(run, "/navigationEndpoint/browseEndpoint/browseId")?;
        if !is_album_run(run, &browse_id) {
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
        let text = run.pointer("/text").and_then(Value::as_str)?.trim();
        looks_like_year(text).then(|| text.to_owned())
    })
}

fn parse_views(runs: &[Value]) -> Option<String> {
    runs.iter().find_map(|run| {
        let text = run.pointer("/text").and_then(Value::as_str)?;
        looks_like_views(text.trim()).then(|| text.trim().to_owned())
    })
}

fn parse_like_status(renderer: &Value) -> Option<LibraryLikeStatus> {
    let items = renderer
        .pointer("/menu/menuRenderer/items")
        .and_then(Value::as_array)?;

    for item in items {
        let Some(toggle) = item.get("toggleMenuServiceItemRenderer") else {
            continue;
        };

        if let Some(status) = optional_text(toggle, "/toggledServiceEndpoint/likeEndpoint/status")
            .as_deref()
            .and_then(parse_like_status_value)
        {
            return Some(status);
        }

        if let Some(status) = optional_text(toggle, "/defaultServiceEndpoint/likeEndpoint/status")
            .as_deref()
            .and_then(infer_like_status_from_default_endpoint)
        {
            return Some(status);
        }
    }

    None
}

fn parse_in_library(renderer: &Value) -> Option<bool> {
    let items = renderer
        .pointer("/menu/menuRenderer/items")
        .and_then(Value::as_array)?;

    for item in items {
        let Some(toggle) = item.get("toggleMenuServiceItemRenderer") else {
            continue;
        };

        if let Some(icon_type) = optional_text(toggle, "/defaultIcon/iconType") {
            match icon_type.as_str() {
                "BOOKMARK" => return Some(true),
                "BOOKMARK_BORDER" => return Some(false),
                _ => continue,
            }
        }
    }

    None
}

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("watch response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("watch response missing {pointer}")))
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
        || browse_id.starts_with("FEmusic_")
        || browse_id.contains("release_detail")
}

fn looks_like_year(text: &str) -> bool {
    text.len() == 4 && text.bytes().all(|byte| byte.is_ascii_digit())
}

fn looks_like_views(text: &str) -> bool {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let Some(first) = normalized.chars().next() else {
        return false;
    };
    let Some(last) = normalized.rsplit(' ').next() else {
        return false;
    };

    first.is_ascii_digit() && matches!(last.to_ascii_lowercase().as_str(), "view" | "views")
}

fn parse_like_status_value(status: &str) -> Option<LibraryLikeStatus> {
    match status {
        "LIKE" => Some(LibraryLikeStatus::Like),
        "INDIFFERENT" => Some(LibraryLikeStatus::Indifferent),
        "DISLIKE" => Some(LibraryLikeStatus::Dislike),
        _ => None,
    }
}

fn infer_like_status_from_default_endpoint(status: &str) -> Option<LibraryLikeStatus> {
    match status {
        "LIKE" => Some(LibraryLikeStatus::Indifferent),
        "INDIFFERENT" => Some(LibraryLikeStatus::Like),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_watch_playlist_continuation, parse_watch_playlist_response, parse_watch_track,
    };
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
        assert_eq!(
            page.continuation,
            Some(ContinuationToken::new("radio-watch-token-1").unwrap())
        );
        assert_eq!(page.items[0].title, "Radio Primary Song");
        assert_eq!(
            page.items[0].like_status,
            Some(LibraryLikeStatus::Indifferent)
        );
        assert_eq!(page.items[0].artists[0].id, "");
        assert_eq!(
            page.items[0].album.as_ref().map(|album| album.id.as_str()),
            Some("release_detail_radio_album")
        );
        assert_eq!(page.items[0].year.as_deref(), Some("1999"));
        assert_eq!(
            page.items[0]
                .counterpart
                .as_ref()
                .and_then(|track| track.views.as_deref()),
            Some("1 view")
        );
    }

    #[test]
    fn parse_watch_playlist_response_handles_shuffle_fixture() {
        let response: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/watch/raw/shuffle_first_page.json"
        ))
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_response(&response).unwrap();
        assert_eq!(
            page.continuation,
            Some(ContinuationToken::new("shuffle-watch-token-1").unwrap())
        );
        assert_eq!(page.items[0].title, "Shuffle Primary Song");
        assert_eq!(
            page.items[0].album.as_ref().map(|album| album.id.as_str()),
            Some("BROWSEshufflealbum1")
        );
        assert_eq!(
            page.items[0].video_type.as_deref(),
            Some("MUSIC_VIDEO_TYPE_OMV")
        );
        assert_eq!(
            page.items[0]
                .counterpart
                .as_ref()
                .and_then(|track| track.views.as_deref()),
            Some("987 views")
        );
    }

    #[test]
    fn parse_watch_playlist_response_handles_plain_queue_thumbnails() {
        let response: serde_json::Value = serde_json::from_str(
            r#"
            {
              "contents": {
                "singleColumnMusicWatchNextResultsRenderer": {
                  "tabbedRenderer": {
                    "watchNextTabbedResultsRenderer": {
                      "tabs": [
                        {
                          "tabRenderer": {
                            "content": {
                              "musicQueueRenderer": {
                                "content": {
                                  "playlistPanelRenderer": {
                                    "contents": [
                                      {
                                        "playlistPanelVideoRenderer": {
                                          "videoId": "plain-thumb-video",
                                          "title": {
                                            "runs": [
                                              { "text": "Plain Thumb Song" }
                                            ]
                                          },
                                          "thumbnail": {
                                            "thumbnails": [
                                              {
                                                "url": "https://example.com/plain.jpg",
                                                "width": 60,
                                                "height": 60
                                              }
                                            ]
                                          }
                                        }
                                      }
                                    ]
                                  }
                                }
                              }
                            }
                          }
                        }
                      ]
                    }
                  }
                }
              }
            }
            "#,
        )
        .unwrap();

        let page: crate::Page<WatchTrack> = parse_watch_playlist_response(&response).unwrap();

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].video_id, "plain-thumb-video");
        assert_eq!(page.items[0].thumbnails.len(), 1);
        assert_eq!(
            page.items[0].thumbnails[0].url,
            "https://example.com/plain.jpg"
        );
    }

    #[test]
    fn parse_watch_track_sets_library_state_from_menu_icon() {
        let bookmark_track: serde_json::Value = serde_json::from_str(
            r#"
            {
              "videoId": "in-library-video",
              "title": { "runs": [{ "text": "In Library Song" }] },
              "thumbnail": {
                "thumbnails": [
                  { "url": "https://example.com/in-library.jpg", "width": 60, "height": 60 }
                ]
              },
              "menu": {
                "menuRenderer": {
                  "items": [
                    {
                      "toggleMenuServiceItemRenderer": {
                        "defaultIcon": {
                          "iconType": "BOOKMARK"
                        }
                      }
                    }
                  ]
                }
              }
            }
            "#,
        )
        .unwrap();

        let bookmark_border_track: serde_json::Value = serde_json::from_str(
            r#"
            {
              "videoId": "not-in-library-video",
              "title": { "runs": [{ "text": "Not In Library Song" }] },
              "thumbnail": {
                "thumbnails": [
                  { "url": "https://example.com/not-in-library.jpg", "width": 60, "height": 60 }
                ]
              },
              "menu": {
                "menuRenderer": {
                  "items": [
                    {
                      "toggleMenuServiceItemRenderer": {
                        "defaultIcon": {
                          "iconType": "BOOKMARK_BORDER"
                        }
                      }
                    }
                  ]
                }
              }
            }
            "#,
        )
        .unwrap();

        let bookmark = parse_watch_track(&bookmark_track).unwrap();
        let bookmark_border = parse_watch_track(&bookmark_border_track).unwrap();

        assert_eq!(bookmark.is_in_library, Some(true));
        assert_eq!(bookmark_border.is_in_library, Some(false));
    }
}
