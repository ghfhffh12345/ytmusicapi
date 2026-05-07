use serde_json::Value;

use crate::{Error, SavedEpisodeItem, SavedEpisodesPage};

use super::{
    core::{
        continuation_shelf, continuation_shelf_contents, extract_continuation_token, optional_text,
        parse_thumbnails, required_runs_text, required_text, section_message_only_without_subtext,
    },
    songs::{column_title_text, flex_column_runs, looks_like_duration},
};

const SAVED_EPISODES_PLAYLIST_ID: &str = "SE";
const SAVED_EPISODES_TITLE: &str = "Saved Episodes";

pub(crate) fn parse_saved_episodes_response(response: &Value) -> Result<SavedEpisodesPage, Error> {
    let sections = required_array_at(
        response,
        "/contents/twoColumnBrowseResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents",
        "library response missing saved episodes sections",
    )?;
    let header = sections
        .iter()
        .find_map(|section| section.get("musicResponsiveHeaderRenderer"))
        .ok_or_else(|| Error::Parse("library response missing saved episodes header".to_owned()))?;
    let items = shelf_contents_or_empty(sections, "library response missing saved episodes items")?;

    Ok(SavedEpisodesPage {
        playlist_id: SAVED_EPISODES_PLAYLIST_ID.to_owned(),
        title: required_runs_text(header, "/title/runs")?,
        items: items
            .iter()
            .map(parse_saved_episode_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: parse_thumbnails(header)?,
        continuation: shelf_continuation_or_empty(
            sections,
            "library response missing saved episodes items",
        )?,
    })
}

pub(crate) fn parse_saved_episodes_continuation(
    response: &Value,
) -> Result<SavedEpisodesPage, Error> {
    Ok(SavedEpisodesPage {
        playlist_id: SAVED_EPISODES_PLAYLIST_ID.to_owned(),
        title: SAVED_EPISODES_TITLE.to_owned(),
        items: continuation_shelf_contents(response)?
            .iter()
            .map(parse_saved_episode_item)
            .collect::<Result<Vec<_>, _>>()?,
        thumbnails: vec![],
        continuation: extract_continuation_token(continuation_shelf(response)?, |token| {
            crate::SavedEpisodesContinuationToken::new(token)
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
        saw_message_only_section |= section_empty_saved_episodes_message(section);
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
) -> Result<Option<crate::SavedEpisodesContinuationToken>, Error> {
    let mut saw_message_only_section = false;
    let mut saw_non_header_section = false;

    for section in sections {
        if let Some(renderer) = section.get("musicPlaylistShelfRenderer") {
            return Ok(extract_continuation_token(renderer, |token| {
                crate::SavedEpisodesContinuationToken::new(token)
            }));
        }

        let is_header_section = section.get("musicResponsiveHeaderRenderer").is_some();
        saw_non_header_section |= !is_header_section;
        saw_message_only_section |= section_empty_saved_episodes_message(section);
    }

    if saw_message_only_section || !saw_non_header_section {
        return Ok(None);
    }

    Err(Error::Parse(missing_message.to_owned()))
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
        .map(flex_column_runs)
        .unwrap_or(&[]);
    let metadata = parse_episode_metadata(subtitle_runs)?;

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

fn parse_episode_metadata(runs: &[Value]) -> Result<ParsedEpisodeMetadata, Error> {
    let mut metadata_values = Vec::new();
    let mut duration = None;

    for value in runs
        .iter()
        .filter_map(|run| optional_text(run, "/text"))
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty() && text != "•")
    {
        if looks_like_duration(&value) {
            if duration.replace(value).is_some() {
                return Err(Error::Parse(
                    "library response has ambiguous saved episode duration metadata".to_owned(),
                ));
            }
        } else {
            metadata_values.push(value);
        }
    }

    let [channel, podcast] = metadata_values.as_slice() else {
        return Err(Error::Parse(
            "library response missing saved episode channel or podcast metadata".to_owned(),
        ));
    };

    Ok(ParsedEpisodeMetadata {
        channel: channel.clone(),
        podcast: podcast.clone(),
        duration,
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

fn flex_columns(renderer: &Value) -> &[Value] {
    renderer
        .get("flexColumns")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn section_empty_saved_episodes_message(section: &Value) -> bool {
    section_message_only_without_subtext(section)
}
