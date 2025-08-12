use rocket::tokio::sync::RwLock;
use std::env;
use std::sync::Arc;

use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::models::tags_data::TagsData;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;
use jukectl_server::mpd_conn::mpd_pool::MpdConnectionPool;

#[derive(Clone)]
pub struct AppState {
    pub mpd_conn: Arc<RwLock<MpdConn>>,
    pub mpd_pool: Arc<MpdConnectionPool>,
    pub song_queue: Arc<RwLock<SongQueue>>,
    pub tags_data: Arc<RwLock<TagsData>>,
    pub album_aware: Arc<RwLock<bool>>, // Album-aware mode flag
}

pub fn initialize() -> AppState {
    // Get MPD configuration from environment
    let mpd_host = env::var("MPD_HOST").unwrap_or_else(|_| "localhost".to_string());
    let mpd_port: u16 = env::var("MPD_PORT")
        .unwrap_or_else(|_| "6600".to_string())
        .parse()
        .expect("Failed to parse MPD_PORT as u16");

    // Initialize the connection pool with the same host/port
    let max_connections = env::var("MPD_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    // Initialize tokio synchronization primitives
    // TODO: this gets MPD_HOST and MPD_PORD deeper in runtime which is a bug and sucks
    // but also everything is moving to pool anyways...
    let mpd_conn = Arc::new(RwLock::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));

    let mpd_pool = Arc::new(MpdConnectionPool::new(&mpd_host, mpd_port, max_connections));

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
        mpd_pool,
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
