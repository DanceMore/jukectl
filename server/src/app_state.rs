use rocket::tokio::sync::RwLock;
use std::sync::Arc;

use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::models::tags_data::TagsData;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;

#[derive(Clone)]
pub struct AppState {
    pub mpd_conn: Arc<RwLock<MpdConn>>,
    pub song_queue: Arc<RwLock<SongQueue>>,
    pub tags_data: Arc<RwLock<TagsData>>,
    pub album_aware: Arc<RwLock<bool>>, // Album-aware mode flag
}

pub fn initialize() -> AppState {
    // Initialize tokio synchronization primitives
    let mpd_conn = Arc::new(RwLock::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));

    let song_queue = Arc::new(RwLock::new(SongQueue::new()));

    // Shareable TagsData with default values including album-aware settings
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(RwLock::new(default_tags_data));

    let album_aware = Arc::new(RwLock::new(false));

    AppState {
        mpd_conn,
        song_queue,
        tags_data,
        album_aware,
    }
}

pub async fn initialize_queue(state: &AppState) {
    let mut locked_mpd_conn = state.mpd_conn.write().await;
    let mut locked_song_queue = state.song_queue.write().await;
    let locked_tags_data = state.tags_data.read().await;
    let locked_album_aware = state.album_aware.read().await;

    println!("[+] Initializing song queue...");

    locked_song_queue.set_album_aware(*locked_album_aware);

    // Use the new caching method for initial queue setup
    // This will build the cache for the first time
    locked_song_queue.shuffle_and_add_with_cache(&*locked_tags_data, &mut *locked_mpd_conn);
}
