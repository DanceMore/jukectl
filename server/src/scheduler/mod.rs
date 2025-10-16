use rocket::tokio;
use rocket::tokio::time::Duration;
use std::io::Write;

use crate::app_state::AppState;

use log::{debug, error, info, trace, warn};

pub async fn start_scheduler(app_state: AppState) {
    info!("[+] Starting scheduler...");
    tokio::spawn(scheduler_mainbody(app_state));
}

async fn scheduler_mainbody(app_state: AppState) {
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
                eprintln!("[!] Error getting MPD connection from pool: {}", e);
                tokio::time::sleep(Duration::from_secs(3)).await;
                continue;
            }
        };

        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // Check if SongQueue is empty and refill if needed
        if locked_song_queue.len() == 0 {
            info!("[!] Scheduler: Queue empty, refilling...");
            locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());
        }

        // Main MPD queue management
        let mpd_queue_result = pooled_conn.mpd_conn().mpd.queue();

        match mpd_queue_result {
            Ok(queue) => {
                let now_playing_len = queue.len();
                if now_playing_len < 2 {
                    let album_aware = *app_state.album_aware.read().await;

                    if album_aware {
                        // Album-aware mode: dequeue full album
                        if let Some(album_songs) =
                            locked_song_queue.remove_album_aware(pooled_conn.mpd_conn())
                        {
                            info!(
                                "[+] Album-aware: Adding {} songs from album",
                                album_songs.len()
                            );

                            for song in album_songs {
                                if let Err(error) = pooled_conn.mpd_conn().mpd.push(song.clone()) {
                                    eprintln!("[!] Error pushing song to MPD: {}", error);
                                } else {
                                    debug!("[+] Added: {}", song.file);
                                }
                            }

                            let _ = pooled_conn.mpd_conn().mpd.play();
                        }
                    } else {
                        // Regular mode: dequeue single song
                        if let Some(song) = locked_song_queue.remove() {
                            if let Err(error) = pooled_conn.mpd_conn().mpd.push(song.clone()) {
                                eprintln!("[!] Error pushing song to MPD: {}", error);
                            } else {
                                info!("[+] Scheduler adding song {}", song.file);
                                let _ = pooled_conn.mpd_conn().mpd.play();
                            }
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("[!] Error getting MPD queue: {}", error);
            }
        }

        // Print cache stats periodically
        if scheduler_cycle % 100 == 0 {
            let (hits, misses, hit_rate) = locked_song_queue.cache_stats();
            let cache_valid = locked_song_queue.has_valid_cache(&*locked_tags_data);
            info!(
                "[+] Cache stats - Hits: {}, Misses: {}, Hit rate: {:.1}%, Valid: {}, Queue: {}",
                hits, misses, hit_rate, cache_valid, locked_song_queue.len()
            );
        }

        drop(locked_song_queue);
        drop(locked_tags_data);

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
