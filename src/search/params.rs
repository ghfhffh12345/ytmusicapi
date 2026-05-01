use crate::model::search::SearchFilter;

pub fn encode_search_params(filter: Option<SearchFilter>, ignore_spelling: bool) -> Option<String> {
    match (filter, ignore_spelling) {
        (None, false) => None,
        (None, true) => Some("EhGKAQ4IARABGAEgASgAOAFAAUICCAE%3D".to_owned()),
        (Some(SearchFilter::Songs), false) => Some("EgWKAQIIAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Songs), true) => {
            Some("EgWKAQIIAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned())
        }
        (Some(SearchFilter::Videos), false) => Some("EgWKAQIQAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Videos), true) => {
            Some("EgWKAQIQAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned())
        }
        (Some(SearchFilter::Albums), false) => Some("EgWKAQIYAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Albums), true) => {
            Some("EgWKAQIYAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned())
        }
        (Some(SearchFilter::Artists), false) => Some("EgWKAQIgAWoMEA4QChADEAQQCRAF".to_owned()),
        (Some(SearchFilter::Artists), true) => {
            Some("EgWKAQIgAUICCAFqDBAOEAoQAxAEEAkQBQ%3D%3D".to_owned())
        }
        (Some(SearchFilter::Playlists), false) => {
            Some("Eg-KAQwIABAAGAAgACgBMABqChAEEAMQCRAFEAo%3D".to_owned())
        }
        (Some(SearchFilter::Playlists), true) => {
            Some("Eg-KAQwIABAAGAAgACgBMABCAggBagoQBBADEAkQBRAK".to_owned())
        }
    }
}
