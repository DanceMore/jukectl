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
}

pub fn initialize() -> AppState {
    // Initialize tokio synchronization primitives
    let mpd_conn = Arc::new(RwLock::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));

    let song_queue = Arc::new(RwLock::new(SongQueue::new()));

    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(RwLock::new(default_tags_data));

    AppState {
        mpd_conn,
        song_queue,
        tags_data,
    }
}

pub async fn initialize_queue(state: &AppState) {
    // Acquire locks for initial setup
    let mut locked_mpd_conn = state.mpd_conn.write().await;
    let mut locked_song_queue = state.song_queue.write().await;
    let locked_tags_data = state.tags_data.read().await;

    // Set up the jukebox SongQueue at boot
    println!("[+] Initializing song queue...");
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
    locked_song_queue.shuffle_and_add(songs);

    // Locks are automatically released when they go out of scope
}
