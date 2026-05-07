use serde_json::Value;

use crate::{
    Error, GetSongResponse, SongByteRange, SongColorInfo, SongMicroformat, SongPlayabilityStatus,
    SongStreamFormat, SongStreamingData, SongVideoDetails, Thumbnail,
};

pub(crate) fn parse_get_song_response(response: &Value) -> Result<GetSongResponse, Error> {
    Ok(GetSongResponse {
        video_details: parse_video_details(required_value(response, "/videoDetails")?)?,
        playability_status: parse_playability_status(required_value(
            response,
            "/playabilityStatus",
        )?)?,
        streaming_data: parse_streaming_data(required_value(response, "/streamingData")?)?,
        microformat: response
            .pointer("/microformat/microformatDataRenderer")
            .map(parse_microformat)
            .transpose()?,
    })
}

fn parse_video_details(value: &Value) -> Result<SongVideoDetails, Error> {
    Ok(SongVideoDetails {
        video_id: required_text(value, "/videoId")?,
        title: required_text(value, "/title")?,
        length_seconds: required_u32_from_str(value, "/lengthSeconds")?,
        channel_id: required_text(value, "/channelId")?,
        author: required_text(value, "/author")?,
        thumbnails: parse_thumbnails(value)?,
        allow_ratings: required_bool(value, "/allowRatings")?,
        view_count: required_text(value, "/viewCount")?,
        is_owner_viewing: required_bool(value, "/isOwnerViewing")?,
        is_crawlable: required_bool(value, "/isCrawlable")?,
        is_private: required_bool(value, "/isPrivate")?,
        is_unplugged_corpus: required_bool(value, "/isUnpluggedCorpus")?,
        is_live_content: required_bool(value, "/isLiveContent")?,
        is_tvfilm_video: required_bool(value, "/isTvfilmVideo")?,
        music_video_type: optional_text(value, "/musicVideoType"),
    })
}

fn parse_playability_status(value: &Value) -> Result<SongPlayabilityStatus, Error> {
    Ok(SongPlayabilityStatus {
        status: required_text(value, "/status")?,
        playable_in_embed: required_bool(value, "/playableInEmbed")?,
        context_params: optional_text(value, "/contextParams"),
        audio_only_availability: optional_text(
            value,
            "/audioOnlyPlayability/audioOnlyPlayabilityRenderer/audioOnlyAvailability",
        ),
        playback_mode: optional_text(value, "/miniplayer/miniplayerRenderer/playbackMode"),
    })
}

fn parse_streaming_data(value: &Value) -> Result<SongStreamingData, Error> {
    Ok(SongStreamingData {
        expires_in_seconds: required_u64_from_str(value, "/expiresInSeconds")?,
        server_abr_streaming_url: optional_text(value, "/serverAbrStreamingUrl"),
        formats: required_array(value, "/formats")?
            .iter()
            .map(parse_stream_format)
            .collect::<Result<Vec<_>, _>>()?,
        adaptive_formats: required_array(value, "/adaptiveFormats")?
            .iter()
            .map(parse_stream_format)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parse_stream_format(value: &Value) -> Result<SongStreamFormat, Error> {
    Ok(SongStreamFormat {
        itag: required_u32(value, "/itag")?,
        mime_type: required_text(value, "/mimeType")?,
        bitrate: required_u64(value, "/bitrate")?,
        average_bitrate: optional_u64(value, "/averageBitrate")?,
        content_length: optional_u64_from_str(value, "/contentLength")?,
        last_modified: optional_text(value, "/lastModified"),
        quality: optional_text(value, "/quality"),
        quality_label: optional_text(value, "/qualityLabel"),
        quality_ordinal: optional_text(value, "/qualityOrdinal"),
        projection_type: optional_text(value, "/projectionType"),
        width: optional_u32(value, "/width")?,
        height: optional_u32(value, "/height")?,
        fps: optional_u32(value, "/fps")?,
        color_info: value
            .pointer("/colorInfo")
            .map(parse_color_info)
            .transpose()?,
        audio_quality: optional_text(value, "/audioQuality"),
        audio_sample_rate: optional_u32_from_str(value, "/audioSampleRate")?,
        audio_channels: optional_u32(value, "/audioChannels")?,
        loudness_db: optional_f64(value, "/loudnessDb"),
        track_absolute_loudness_lkfs: optional_f64(value, "/trackAbsoluteLoudnessLkfs"),
        approx_duration_ms: optional_u64_from_str(value, "/approxDurationMs")?,
        high_replication: value.pointer("/highReplication").and_then(Value::as_bool),
        xtags: optional_text(value, "/xtags"),
        init_range: value
            .pointer("/initRange")
            .map(parse_byte_range)
            .transpose()?,
        index_range: value
            .pointer("/indexRange")
            .map(parse_byte_range)
            .transpose()?,
        signature_cipher: required_text(value, "/signatureCipher")?,
    })
}

fn parse_thumbnails(value: &Value) -> Result<Vec<Thumbnail>, Error> {
    required_array(value, "/thumbnail/thumbnails")?
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

fn parse_byte_range(value: &Value) -> Result<SongByteRange, Error> {
    Ok(SongByteRange {
        start: required_text(value, "/start")?,
        end: required_text(value, "/end")?,
    })
}

fn parse_color_info(value: &Value) -> Result<SongColorInfo, Error> {
    let _ = required_value(value, "")?;

    Ok(SongColorInfo {
        primaries: optional_text(value, "/primaries"),
        transfer_characteristics: optional_text(value, "/transferCharacteristics"),
        matrix_coefficients: optional_text(value, "/matrixCoefficients"),
    })
}

fn parse_microformat(value: &Value) -> Result<SongMicroformat, Error> {
    Ok(SongMicroformat {
        url_canonical: optional_text(value, "/urlCanonical"),
        description: optional_text(value, "/description"),
        category: optional_text(value, "/category"),
        publish_date: optional_text(value, "/publishDate"),
        upload_date: optional_text(value, "/uploadDate"),
        view_count: optional_text(value, "/viewCount"),
        available_countries: optional_string_array(value, "/availableCountries")?,
        tags: optional_string_array(value, "/tags")?,
        noindex: value.pointer("/noindex").and_then(Value::as_bool),
        unlisted: value.pointer("/unlisted").and_then(Value::as_bool),
        family_safe: value.pointer("/familySafe").and_then(Value::as_bool),
    })
}

fn required_value<'a>(value: &'a Value, pointer: &str) -> Result<&'a Value, Error> {
    value
        .pointer(pointer)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
}

fn required_array<'a>(value: &'a Value, pointer: &str) -> Result<&'a [Value], Error> {
    required_value(value, pointer)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
}

fn required_text(value: &Value, pointer: &str) -> Result<String, Error> {
    optional_text(value, pointer)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
}

fn optional_text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn required_bool(value: &Value, pointer: &str) -> Result<bool, Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
}

fn required_u32(value: &Value, pointer: &str) -> Result<u32, Error> {
    let number = value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))?;

    u32::try_from(number).map_err(|_| Error::Parse(format!("song response missing {pointer}")))
}

fn optional_u32(value: &Value, pointer: &str) -> Result<Option<u32>, Error> {
    value
        .pointer(pointer)
        .map(|number| {
            let number = number
                .as_u64()
                .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))?;

            u32::try_from(number)
                .map_err(|_| Error::Parse(format!("song response missing {pointer}")))
        })
        .transpose()
}

fn required_u64(value: &Value, pointer: &str) -> Result<u64, Error> {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
}

fn optional_u64(value: &Value, pointer: &str) -> Result<Option<u64>, Error> {
    value
        .pointer(pointer)
        .map(|number| {
            number
                .as_u64()
                .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
        })
        .transpose()
}

fn required_u32_from_str(value: &Value, pointer: &str) -> Result<u32, Error> {
    required_text(value, pointer)?
        .parse()
        .map_err(|_| Error::Parse(format!("song response missing {pointer}")))
}

fn optional_u32_from_str(value: &Value, pointer: &str) -> Result<Option<u32>, Error> {
    optional_text(value, pointer)
        .map(|number| {
            number
                .parse()
                .map_err(|_| Error::Parse(format!("song response missing {pointer}")))
        })
        .transpose()
}

fn required_u64_from_str(value: &Value, pointer: &str) -> Result<u64, Error> {
    required_text(value, pointer)?
        .parse()
        .map_err(|_| Error::Parse(format!("song response missing {pointer}")))
}

fn optional_u64_from_str(value: &Value, pointer: &str) -> Result<Option<u64>, Error> {
    optional_text(value, pointer)
        .map(|number| {
            number
                .parse()
                .map_err(|_| Error::Parse(format!("song response missing {pointer}")))
        })
        .transpose()
}

fn optional_f64(value: &Value, pointer: &str) -> Option<f64> {
    value.pointer(pointer).and_then(Value::as_f64)
}

fn optional_string_array(value: &Value, pointer: &str) -> Result<Vec<String>, Error> {
    let Some(values) = value.pointer(pointer) else {
        return Ok(Vec::new());
    };

    let values = values
        .as_array()
        .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))?;

    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| Error::Parse(format!("song response missing {pointer}")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::parse_get_song_response;
    use crate::Error;

    #[test]
    fn parse_get_song_response_parses_response1_payload() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/song/raw/response1.json"))
                .unwrap();

        let parsed = parse_get_song_response(&response).unwrap();

        assert_eq!(parsed.video_details.video_id, "0rilIYWiJ7M");
        assert_eq!(parsed.video_details.length_seconds, 267);
        assert_eq!(parsed.playability_status.status, "OK");
        assert_eq!(parsed.streaming_data.formats.len(), 1);
        assert_eq!(parsed.streaming_data.adaptive_formats.len(), 18);
        assert_eq!(
            parsed.streaming_data.formats[0].signature_cipher,
            response["streamingData"]["formats"][0]["signatureCipher"]
                .as_str()
                .unwrap()
        );
        assert_eq!(
            parsed
                .microformat
                .as_ref()
                .unwrap()
                .url_canonical
                .as_deref(),
            Some(
                "https://music.youtube.com/watch?v=0rilIYWiJ7M&list=OLAK5uy_kvpvj0lyrvglbhtsIDAi_dHoEGrJRfwuk&index=0"
            )
        );
    }

    #[test]
    fn parse_get_song_response_preserves_optional_stream_fields_from_response2() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/song/raw/response2.json"))
                .unwrap();

        let parsed = parse_get_song_response(&response).unwrap();

        assert_eq!(
            parsed.streaming_data.formats[0].content_length,
            Some(2_399_880)
        );
        assert_eq!(
            parsed.streaming_data.formats[0].xtags.as_deref(),
            Some("Cg8KB2hlYXVkaW8SBHRydWU")
        );
        assert_eq!(
            parsed.playability_status.audio_only_availability.as_deref(),
            Some("FEATURE_AVAILABILITY_ALLOWED")
        );
    }

    #[test]
    fn parse_get_song_response_parses_audio_only_adaptive_fields_from_response3() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/song/raw/response3.json"))
                .unwrap();

        let parsed = parse_get_song_response(&response).unwrap();
        let audio = parsed
            .streaming_data
            .adaptive_formats
            .iter()
            .find(|format| format.itag == 140)
            .unwrap();

        assert_eq!(parsed.streaming_data.adaptive_formats.len(), 16);
        assert_eq!(audio.audio_quality.as_deref(), Some("AUDIO_QUALITY_MEDIUM"));
        assert_eq!(audio.audio_channels, Some(2));
        assert_eq!(audio.high_replication, Some(true));
        assert_eq!(
            audio.init_range.as_ref().map(|range| range.start.as_str()),
            Some("0")
        );
        assert_eq!(audio.track_absolute_loudness_lkfs, Some(-7.96));
    }

    #[test]
    fn parse_get_song_response_rejects_missing_streaming_data() {
        let response = serde_json::json!({
            "videoDetails": {
                "videoId": "video-1",
                "title": "Track Title",
                "lengthSeconds": "245",
                "channelId": "UC123",
                "thumbnail": { "thumbnails": [] },
                "allowRatings": true,
                "viewCount": "42",
                "author": "Artist",
                "isOwnerViewing": false,
                "isCrawlable": true,
                "isPrivate": false,
                "isUnpluggedCorpus": false,
                "isLiveContent": false,
                "isTvfilmVideo": false
            },
            "playabilityStatus": {
                "status": "OK",
                "playableInEmbed": true
            }
        });

        let err = parse_get_song_response(&response).unwrap_err();
        assert!(matches!(
            err,
            Error::Parse(message) if message == "song response missing /streamingData"
        ));
    }
}
