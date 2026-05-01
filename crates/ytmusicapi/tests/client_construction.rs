use ytmusicapi::YtMusic;

#[test]
fn constructs_default_client() {
    let client = YtMusic::new().expect("client should build");
    let debug = format!("{client:?}");
    assert!(debug.contains("YtMusic"));
}
