use ytmusicapi::{SearchResult, parse_search_response_for_test};

fn result_type(result: &SearchResult) -> &'static str {
    match result {
        SearchResult::Song(result) => result.result_type.as_str(),
        SearchResult::Video(result) => result.result_type.as_str(),
        SearchResult::Album(result) => result.result_type.as_str(),
        SearchResult::Artist(result) => result.result_type.as_str(),
        SearchResult::Profile(result) => result.result_type.as_str(),
        SearchResult::Playlist(result) => result.result_type.as_str(),
    }
}

#[test]
fn default_mixed_fixture_matches_expected_top_level_types() {
    let response: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/search/raw/default_mixed.json")).unwrap();

    let parsed = parse_search_response_for_test(&response, None).unwrap();

    let top_level_types: Vec<_> = parsed.iter().map(result_type).collect();
    assert_eq!(
        top_level_types,
        vec![
            "artist", "album", "album", "album", "artist", "playlist", "artist", "artist",
            "playlist", "playlist", "episode", "episode", "episode", "episode", "playlist",
            "episode", "profile", "profile", "profile", "playlist", "playlist", "podcast",
            "podcast", "podcast",
        ]
    );

    assert!(matches!(
        &parsed[0],
        SearchResult::Artist(result)
            if result.category.as_deref() == Some("Top result")
                && result.artists[0].name == "Daft Punk"
                && result.subscribers.as_deref() == Some("75.2M")
    ));
    assert!(matches!(
        &parsed[1],
        SearchResult::Album(result)
            if result.title == "Random Access Memories"
                && result.artists[0].name == "Daft Punk"
                && result.thumbnails.len() == 4
    ));
    assert!(matches!(
        &parsed[5],
        SearchResult::Playlist(result)
            if result.title == "Best Of Daft Punk"
                && result.author.as_deref() == Some("misterepicpants")
                && result.item_count.is_none()
    ));
    assert!(matches!(
        &parsed[8],
        SearchResult::Playlist(result)
            if result.title == "Daft Punk Greatest Hits"
                && result.author.as_deref() == Some("Brandon G")
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
        &parsed[17],
        SearchResult::Profile(result)
            if result.name == "Daft punk discovery1234"
                && result.handle == "@Djismaill902"
                && result.browse_id == "UCMAe-IuKNTtLPlZyaqv4fXQ"
    ));
    assert!(matches!(
        &parsed[18],
        SearchResult::Profile(result)
            if result.name == "DAFT PUNK ME"
                && result.handle == "@claudiachuraaruquipa"
                && result.browse_id == "UCQh3zCs8lE-WWQemrUMrlsg"
    ));
    assert!(matches!(
        &parsed[10],
        SearchResult::Video(result)
            if result.result_type.as_str() == "episode"
                && result
                    .podcast
                    .as_ref()
                    .map(|podcast| podcast.name.as_str())
                    == Some("🌟 Funkzone Sound – Future Funk Podcast 🌟")
    ));
    assert!(matches!(
        &parsed[21],
        SearchResult::Playlist(result)
            if result.result_type.as_str() == "podcast"
                && result.title == "off Track Podcast Season 2"
    ));
}
