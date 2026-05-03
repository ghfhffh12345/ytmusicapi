use serde_json::Value;

use crate::{Error, SavedEpisodeItem, SavedEpisodes};

use super::core::{optional_text, parse_thumbnails, required_runs_text, required_text};

pub(crate) fn parse_saved_episodes_response(response: &Value) -> Result<SavedEpisodes, Error> {
    let header = required_value_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicResponsiveHeaderRenderer",
        "library response missing saved episodes header",
    )?;
    let items = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/1/musicPlaylistShelfRenderer/contents",
        "library response missing saved episodes items",
    )?;

    Ok(SavedEpisodes {
        playlist_id: "SE".to_owned(),
        title: required_runs_text(header, "/title/runs")?,
        items: items
            .iter()
            .map(parse_saved_episode_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: parse_thumbnails(header)?,
    })
}

fn parse_saved_episode_item(item: &Value) -> Result<SavedEpisodeItem, Error> {
    let renderer = item.get("musicResponsiveListItemRenderer").ok_or_else(|| {
        Error::Parse(
            "library response missing musicResponsiveListItemRenderer in saved episodes item"
                .to_owned(),
        )
    })?;
    let title = flex_columns(renderer)
        .first()
        .and_then(column_title_text)
        .ok_or_else(|| Error::Parse("library response missing saved episode title".to_owned()))?;
    let subtitle_runs = flex_columns(renderer)
        .get(1)
        .map(|column| flex_column_runs(column))
        .unwrap_or(&[]);
    let metadata = parse_episode_metadata(subtitle_runs);

    Ok(SavedEpisodeItem {
        video_id: required_text(renderer, "/playlistItemData/videoId")?,
        title,
        channel: metadata.channel,
        podcast: metadata.podcast,
        duration: metadata.duration,
        thumbnails: parse_thumbnails(renderer)?,
    })
}

struct ParsedEpisodeMetadata {
    channel: String,
    podcast: String,
    duration: Option<String>,
}

fn parse_episode_metadata(runs: &[Value]) -> ParsedEpisodeMetadata {
    let values = runs
        .iter()
        .filter_map(|run| optional_text(run, "/text"))
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty() && text != "•")
        .collect::<Vec<_>>();

    let mut channel = String::new();
    let mut podcast = String::new();
    let mut duration = None;

    for value in values {
        if duration.is_none() && looks_like_duration(&value) {
            duration = Some(value);
            continue;
        }

        if channel.is_empty() {
            channel = value;
            continue;
        }

        if podcast.is_empty() {
            podcast = value;
        }
    }

    ParsedEpisodeMetadata {
        channel,
        podcast,
        duration,
    }
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

fn flex_columns(renderer: &Value) -> &[Value] {
    renderer
        .get("flexColumns")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn flex_column_runs(column: &Value) -> &[Value] {
    column
        .pointer("/musicResponsiveListItemFlexColumnRenderer/text/runs")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn column_title_text(column: &Value) -> Option<String> {
    flex_column_runs(column).iter().find_map(|run| {
        let text = optional_text(run, "/text")?;
        let has_watch_endpoint = run.pointer("/navigationEndpoint/watchEndpoint").is_some();
        has_watch_endpoint.then_some(text)
    })
}

fn looks_like_duration(text: &str) -> bool {
    let parts = text.split(':').collect::<Vec<_>>();
    if !(parts.len() == 2 || parts.len() == 3) {
        return false;
    }

    parts.iter().enumerate().all(|(index, part)| {
        !part.is_empty()
            && part.chars().all(|ch| ch.is_ascii_digit())
            && (index == 0 || part.len() == 2)
    })
}
