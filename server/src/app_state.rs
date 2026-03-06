use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

use crate::models::song_queue::SongQueue;
use crate::models::tags_data::TagsData;
use crate::mpd_conn::mpd_conn::MpdConn;
use crate::mpd_conn::mpd_pool::MpdPool;
use crate::mpd_conn::traits::MpdClient;

pub struct Config {
    pub album_aware_shuffle: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub mpd_pool: Arc<MpdPool>,
    pub queue: Arc<Mutex<SongQueue>>,
    pub config: Arc<Mutex<Config>>,
    pub tags_data: Arc<RwLock<TagsData>>,
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

    let default_tags = load_default_tags();
    let tags_data = Arc::new(RwLock::new(default_tags));

    AppState {
        mpd_pool,
        queue,
        config,
        tags_data,
    }
}

pub async fn initialize_queue(state: &AppState) {
    log::info!("[+] Initializing song queue...");
    
    let mut pooled_conn = match state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(e) => {
            log::error!("[!] Failed to get connection for initialization: {}", e);
            return;
        }
    };

    let mut locked_song_queue = state.queue.lock().await;
    let locked_tags_data = state.tags_data.read().await;
    let locked_config = state.config.lock().await;

    locked_song_queue.set_album_aware(locked_config.album_aware_shuffle);

    // Initial queue fill
    locked_song_queue.shuffle_and_add(&*locked_tags_data, &mut pooled_conn.mpd_conn().mpd);
    
    log::info!("[+] Queue initialization complete. ({} songs)", locked_song_queue.len());
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
