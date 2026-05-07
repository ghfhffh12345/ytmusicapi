use ytmusicapi::{WatchPlaylistQuery, YtMusic};

#[tokio::main]
async fn main() {
    let ytmusic = YtMusic::builder()
        .browser_auth_path("./browser.json")
        .build()
        .unwrap();

    let first_page = ytmusic
        .get_watch_playlist(
            WatchPlaylistQuery::new()
                .with_video_id("LhiRts68_bk")
                .radio(),
        )
        .await
        .unwrap();

    println!("first page items: {}", first_page.items.len());
    println!("has continuation: {}", first_page.continuation.is_some());

    if let Some(track) = first_page.items.first() {
        println!("first track: {} ({})", track.title, track.video_id);
    }

    if let Some(token) = first_page.continuation.clone() {
        let continuation = ytmusic
            .get_watch_playlist_continuation(token)
            .await
            .unwrap();
        println!("continuation items: {}", continuation.items.len());
    }
}
