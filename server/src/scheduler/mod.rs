use tokio::time::Duration;
use std::io::Write;
use std::sync::Arc;

use crate::app_state::AppState;
use crate::mpd_conn::traits::{MpdClient, Song};
use crate::models::song_queue::DequeueMode;

use log::{debug, info, trace, error};

pub async fn start_scheduler(app_state: AppState) {
    info!("[+] Starting scheduler...");
    let app_state_arc = Arc::new(app_state);
    tokio::spawn(scheduler_mainbody(app_state_arc));
}

async fn scheduler_mainbody(app_state: Arc<AppState>) {
    let mut scheduler_cycle = 0u64;

    loop {
        scheduler_cycle += 1;

        if scheduler_cycle % 20 == 0 {
            trace!("[-] scheduler cycle #{}", scheduler_cycle);
        } else {
            print!(".");
            let _ = std::io::stdout().flush();
        }

        let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                error!("[!] Error getting MPD connection from pool: {}", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        };

        let mpd_queue_result: anyhow::Result<Vec<Song>> = pooled_conn.mpd_conn().mpd.queue();

        match mpd_queue_result {
            Ok(queue) => {
                if queue.len() < 2 {
                    let mut locked_song_queue = app_state.queue.lock().await;
                    
                    if !locked_song_queue.is_empty() {
                        let mode = if app_state.config.lock().await.album_aware_shuffle {
                            DequeueMode::Album
                        } else {
                            DequeueMode::Single
                        };
                        
                        let songs = locked_song_queue.dequeue(mode, &mut pooled_conn);

                        if !songs.is_empty() {
                            info!("[+] Scheduler adding {} song(s) to MPD queue", songs.len());

                            for song in songs {
                                if let Err(err) = pooled_conn.mpd_conn().mpd.push(&song.file) {
                                    error!("[!] Error pushing song to MPD: {}", err);
                                } else {
                                    debug!("[+] Added: {}", song.file);
                                }
                            }

                            let _ = pooled_conn.mpd_conn().mpd.play();
                        }
                    }
                }
            }
            Err(err) => {
                error!("[!] Error getting MPD queue: {}", err);
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
