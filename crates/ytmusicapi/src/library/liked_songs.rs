use serde_json::Value;

use crate::{Error, LikedSongItem, LikedSongs};

use super::{
    core::{parse_thumbnails, required_runs_text},
    songs::parse_song_list_item,
};

pub(crate) fn parse_liked_songs_response(response: &Value) -> Result<LikedSongs, Error> {
    let header = required_value_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicResponsiveHeaderRenderer",
        "library response missing liked songs header",
    )?;
    let items = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/1/musicPlaylistShelfRenderer/contents",
        "library response missing liked songs items",
    )?;

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

fn required_value_at<'a>(
    value: &'a Value,
    pointer: &str,
    message: &str,
) -> Result<&'a Value, Error> {
    value
        .pointer(pointer)
        .ok_or_else(|| Error::Parse(message.to_owned()))
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
