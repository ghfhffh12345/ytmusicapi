use serde_json::Value;

use crate::model::library::LikedSongs;
use crate::{Error, LikedSongItem};

use super::{
    core::{parse_thumbnails, required_runs_text, section_message_only_without_subtext},
    songs::parse_song_list_item,
};

pub(crate) fn parse_liked_songs_response(response: &Value) -> Result<LikedSongs, Error> {
    let sections = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
        "library response missing liked songs sections",
    )?;
    let header = sections
        .iter()
        .find_map(|section| section.get("musicResponsiveHeaderRenderer"))
        .ok_or_else(|| Error::Parse("library response missing liked songs header".to_owned()))?;
    let items = shelf_contents_or_empty(sections, "library response missing liked songs items")?;

    Ok(LikedSongs {
        playlist_id: "LM".to_owned(),
        title: required_runs_text(header, "/title/runs")?,
        items: items
            .iter()
            .map(parse_liked_song_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: parse_thumbnails(header)?,
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
