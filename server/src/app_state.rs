use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::models::song_queue::SongQueue;
use crate::models::tags_data::TagsData;
use crate::mpd_conn::mpd_conn::MpdConn;
use crate::mpd_conn::mpd_pool::MpdPool;

pub struct Config {
    pub album_aware_shuffle: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub mpd_pool: Arc<MpdPool>,
    pub queue: Arc<Mutex<SongQueue>>,
    pub config: Arc<Mutex<Config>>,
    pub default_tags: TagsData,
}

pub async fn initialize() -> AppState {
    let mpd_host = env::var("MPD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let mpd_port: u16 = env::var("MPD_PORT")
        .unwrap_or_else(|_| "6600".to_string())
        .parse()
        .unwrap_or(6600);

    let max_connections = env::var("MPD_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    let mpd_pool = Arc::new(
        MpdPool::new(mpd_host, mpd_port, max_connections)
            .expect("Failed to create MPD pool"),
    );
    
    let _ = mpd_pool.warm_pool(1).await;

    let queue = Arc::new(Mutex::new(SongQueue::new()));
    let config = Arc::new(Mutex::new(Config {
        album_aware_shuffle: env::var("ALBUM_AWARE_SHUFFLE").unwrap_or_default() == "1",
    }));

    AppState {
        mpd_pool,
        queue,
        config,
        default_tags: load_default_tags(),
    }
}

pub async fn initialize_queue(_state: &AppState) {
    log::info!("Queue initialization complete.");
}

pub fn load_default_tags() -> TagsData {
    let fallback = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };

    match env::var("JUKECTL_DEFAULT_TAGS_B64") {
        Ok(b64_tags) => {
            use base64::{engine::general_purpose, Engine as _};
            match general_purpose::STANDARD.decode(b64_tags) {
                Ok(decoded) => match serde_json::from_slice::<TagsData>(&decoded) {
                    Ok(tags) => tags,
                    Err(e) => {
                        log::warn!("[!] Failed to parse JUKECTL_DEFAULT_TAGS_B64 as JSON: {}", e);
                        fallback
                    }
                },
                Err(e) => {
                    log::warn!("[!] Failed to decode JUKECTL_DEFAULT_TAGS_B64 as Base64: {}", e);
                    fallback
                }
            }
        }
        Err(_) => fallback,
    }
}
