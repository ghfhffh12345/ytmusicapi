use ytmusicapi::{Error, SearchFilter, SearchQuery};

#[test]
fn default_query_omits_params_and_has_no_limit_knob() {
    let query = SearchQuery::new("oasis wonderwall");
    assert_eq!(
        query,
        SearchQuery {
            query: "oasis wonderwall".to_owned(),
            filter: None,
            ignore_spelling: false,
        }
    );
    assert_eq!(query.encoded_params().as_deref(), None);
}

#[test]
fn ignore_spelling_sets_default_params() {
    let query = SearchQuery::new("martin stig andersen - deteriation").ignore_spelling();
    assert_eq!(
        query.encoded_params().as_deref(),
        Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D")
    );
}

#[test]
fn filter_encodings_match_upstream() {
    let cases = [
        (
            SearchFilter::Songs,
            "EgWKAQIIAWoMEA4QChADEAQQCRAF",
            "EgWKAQIIAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Videos,
            "EgWKAQIQAWoMEA4QChADEAQQCRAF",
            "EgWKAQIQAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Albums,
            "EgWKAQIYAWoMEA4QChADEAQQCRAF",
            "EgWKAQIYAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Artists,
            "EgWKAQIgAWoMEA4QChADEAQQCRAF",
            "EgWKAQIgAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D",
        ),
        (
            SearchFilter::Playlists,
            "Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D",
            "Eg-KAQwIABAAGAAgACgBMABCAggBagoQBBADEAkQBRAK",
        ),
    ];

    for (filter, expected, expected_ignore) in cases {
        let filtered = SearchQuery::new("hip hop").with_filter(filter);
        assert_eq!(filtered.encoded_params().as_deref(), Some(expected));

        let ignored = SearchQuery::new("hip hop")
            .with_filter(filter)
            .ignore_spelling();
        assert_eq!(ignored.encoded_params().as_deref(), Some(expected_ignore));
    }
}

#[test]
fn blank_query_is_rejected() {
    let result = SearchQuery::new("   ").validate();
    assert!(matches!(result, Err(Error::InvalidInput(_))));
}

#[test]
fn songs_and_videos_filters_validate() {
    for filter in [SearchFilter::Songs, SearchFilter::Videos] {
        let result = SearchQuery::new("abba").with_filter(filter).validate();
        assert!(result.is_ok(), "{filter:?} should validate successfully");
    }
}
