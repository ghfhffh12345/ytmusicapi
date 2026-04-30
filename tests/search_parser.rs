use ytmusicapi::{SearchResult, internal::parse_search_response_for_test};

#[test]
fn default_mixed_fixture_matches_expected_top_level_types() {
    let response: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/search/raw/default_mixed.json")).unwrap();
    let expected: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/search/expected/default_mixed.json")).unwrap();

    let parsed = parse_search_response_for_test(&response, None).unwrap();
    let actual = serde_json::to_value(&parsed).unwrap();

    assert_eq!(actual, expected);

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
