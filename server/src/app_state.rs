use rocket::tokio::sync::RwLock;
use std::env;
use std::sync::Arc;

use crate::models::song_queue::SongQueue;
use crate::models::tags_data::TagsData;
use crate::mpd_conn::mpd_conn::MpdConn;
use crate::mpd_conn::mpd_pool::MpdConnectionPool;

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};

    #[test]
    fn test_load_default_tags_fallback() {
        env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
        let tags = load_default_tags();
        assert_eq!(tags.any, vec!["jukebox".to_string()]);
        assert_eq!(tags.not, vec!["explicit".to_string()]);
    }

    #[test]
    fn test_load_default_tags_valid_b64() {
        let json = r#"{"any": ["tag1", "tag2"], "not": ["tag3"]}"#;
        let b64 = general_purpose::STANDARD.encode(json);
        env::set_var("JUKECTL_DEFAULT_TAGS_B64", b64);

        let tags = load_default_tags();
        assert_eq!(tags.any, vec!["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(tags.not, vec!["tag3".to_string()]);

        env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
    }

    #[test]
    fn test_load_default_tags_invalid_b64() {
        env::set_var("JUKECTL_DEFAULT_TAGS_B64", "!!!not-base64!!!");
        let tags = load_default_tags();
        // Should fallback
        assert_eq!(tags.any, vec!["jukebox".to_string()]);

        env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
    }

    #[test]
    fn test_load_default_tags_invalid_json() {
        let invalid_json = r#"{"any": ["tag1"], "not": "#; // missing value
        let b64 = general_purpose::STANDARD.encode(invalid_json);
        env::set_var("JUKECTL_DEFAULT_TAGS_B64", b64);

        let tags = load_default_tags();
        // Should fallback
        assert_eq!(tags.any, vec!["jukebox".to_string()]);

        env::remove_var("JUKECTL_DEFAULT_TAGS_B64");
    }
}

#[derive(Clone)]
pub struct AppState {
    pub mpd_conn: Arc<RwLock<MpdConn>>,
    pub mpd_pool: Arc<MpdConnectionPool>,
    pub song_queue: Arc<RwLock<SongQueue>>,
    pub tags_data: Arc<RwLock<TagsData>>,
    pub album_aware: Arc<RwLock<bool>>,
}

pub async fn initialize() -> AppState {
    // Get MPD configuration from environment
    let mpd_host = env::var("MPD_HOST").unwrap_or_else(|_| "localhost".to_string());
    let mpd_port: u16 = env::var("MPD_PORT")
        .unwrap_or_else(|_| "6600".to_string())
        .parse()
        .expect("Failed to parse MPD_PORT as u16");

    // Initialize the connection pool
    let max_connections = env::var("MPD_MAX_CONNECTIONS")
        .unwrap_or_else(|_| "5".to_string())
        .parse()
        .unwrap_or(5);

    let mpd_conn = Arc::new(RwLock::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));

    // Now pool initialization is async and built-in
    let mpd_pool = Arc::new(
        MpdConnectionPool::new(&mpd_host, mpd_port, max_connections)
            .await
            .expect("Failed to create MPD connection pool"),
    );

    let song_queue = Arc::new(RwLock::new(SongQueue::new()));

    // Default tags
    let default_tags_data = load_default_tags();
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

    // Initial queue fill - uses cache internally
    locked_song_queue.shuffle_and_add(&*locked_tags_data, &mut *locked_mpd_conn);
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
