use serde_json::Value;

use crate::{Error, LikedSongItem, LikedSongsPage};

use super::{
    core::{
        continuation_shelf, continuation_shelf_contents, extract_continuation_token,
        parse_thumbnails, required_runs_text, section_message_only_without_subtext,
    },
    songs::parse_song_list_item,
};

const LIKED_SONGS_PLAYLIST_ID: &str = "LM";
const LIKED_SONGS_TITLE: &str = "Liked Songs";

pub(crate) fn parse_liked_songs_response(response: &Value) -> Result<LikedSongsPage, Error> {
    let header_sections = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
        "library response missing liked songs sections",
    )?;
    let item_sections = response
        .pointer("/contents/twoColumnBrowseResultsRenderer/secondaryContents/sectionListRenderer/contents")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(header_sections);
    let header = header_sections
        .iter()
        .chain(item_sections.iter())
        .find_map(|section| section.get("musicResponsiveHeaderRenderer"))
        .ok_or_else(|| Error::Parse("library response missing liked songs header".to_owned()))?;
    let items =
        shelf_contents_or_empty(item_sections, "library response missing liked songs items")?;

    Ok(LikedSongsPage {
        playlist_id: LIKED_SONGS_PLAYLIST_ID.to_owned(),
        title: required_runs_text(header, "/title/runs")?,
        items: items
            .iter()
            .filter(|item| item.get("continuationItemRenderer").is_none())
            .map(parse_liked_song_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: parse_thumbnails(header)?,
        continuation: shelf_continuation_or_empty(
            item_sections,
            "library response missing liked songs items",
        )?,
    })
}

pub(crate) fn parse_liked_songs_continuation(response: &Value) -> Result<LikedSongsPage, Error> {
    Ok(LikedSongsPage {
        playlist_id: LIKED_SONGS_PLAYLIST_ID.to_owned(),
        title: LIKED_SONGS_TITLE.to_owned(),
        items: continuation_shelf_contents(response)?
            .iter()
            .map(parse_liked_song_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: vec![],
        continuation: extract_continuation_token(continuation_shelf(response)?, |token| {
            crate::LikedSongsContinuationToken::new(token)
        }),
    })
}

fn shelf_contents_or_empty<'a>(
    sections: &'a [Value],
    missing_message: &str,
) -> Result<&'a [Value], Error> {
    let mut saw_message_only_section = false;
    let mut saw_non_header_section = false;

    for section in sections {
        if let Some(contents) = section
            .pointer("/musicPlaylistShelfRenderer/contents")
            .and_then(Value::as_array)
        {
            return Ok(contents.as_slice());
        }

        let is_header_section = section.get("musicResponsiveHeaderRenderer").is_some();
        saw_non_header_section |= !is_header_section;
        saw_message_only_section |= section_empty_liked_songs_message(section);
    }

    if saw_message_only_section {
        return Ok(&[]);
    }

    if !saw_non_header_section {
        return Ok(&[]);
    }

    Err(Error::Parse(missing_message.to_owned()))
}

fn shelf_continuation_or_empty(
    sections: &[Value],
    missing_message: &str,
) -> Result<Option<crate::LikedSongsContinuationToken>, Error> {
    let mut saw_message_only_section = false;
    let mut saw_non_header_section = false;

    for section in sections {
        if let Some(renderer) = section.get("musicPlaylistShelfRenderer") {
            return Ok(extract_continuation_token(renderer, |token| {
                crate::LikedSongsContinuationToken::new(token)
            })
            .or_else(|| continuation_item_token(renderer)));
        }

        let is_header_section = section.get("musicResponsiveHeaderRenderer").is_some();
        saw_non_header_section |= !is_header_section;
        saw_message_only_section |= section_empty_liked_songs_message(section);
    }

    if saw_message_only_section || !saw_non_header_section {
        return Ok(None);
    }

    Err(Error::Parse(missing_message.to_owned()))
}

fn continuation_item_token(renderer: &Value) -> Option<crate::LikedSongsContinuationToken> {
    renderer
        .pointer("/contents")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|item| {
            item.pointer("/continuationItemRenderer/continuationEndpoint/continuationCommand/token")
                .and_then(Value::as_str)
        })
        .map(crate::LikedSongsContinuationToken::new)
}

fn parse_liked_song_item(item: &Value) -> Result<LikedSongItem, Error> {
    parse_song_list_item(item, "liked songs item").map(|parsed| LikedSongItem {
        video_id: parsed.video_id,
        title: parsed.title,
        artists: parsed.artists,
        album: parsed.album,
        duration: parsed.duration,
        thumbnails: parsed.thumbnails,
        like_status: parsed.like_status,
    })
}

fn required_array_at<'a>(
    value: &'a Value,
    pointer: &str,
    message: &str,
) -> Result<&'a [Value], Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(message.to_owned()))
}

fn section_empty_liked_songs_message(section: &Value) -> bool {
    section_message_only_without_subtext(section)
}
