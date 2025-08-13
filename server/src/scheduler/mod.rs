use rocket::tokio;
use rocket::tokio::time::Duration;
use std::io::Write;

use crate::app_state::AppState;

use log::{trace, debug, info, warn, error};


pub async fn start_scheduler(app_state: AppState) {
    info!("[+] Starting enhanced scheduler with background precomputation...");

    // Start both the main scheduler and background precompute task
    let app_state_clone = app_state.clone();
    tokio::spawn(scheduler_mainbody(app_state));
    tokio::spawn(background_precompute_task(app_state_clone));
}

async fn scheduler_mainbody(app_state: AppState) {
    let mut scheduler_cycle = 0u64;

    loop {
        scheduler_cycle += 1;

        if scheduler_cycle % 20 == 0 {
            // Every minute (3s * 20)
            trace!("[-] scheduler cycle #{}", scheduler_cycle);
        } else {
            print!(".");
            let _ = std::io::stdout().flush();
        }

        // Get connection from pool instead of locking single connection
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
            info!("[!] scheduler sees an empty queue, refilling with cached data...");

            // Use the ultra-fast async method for instant refills
            locked_song_queue
                .shuffle_and_add_with_cache_async(&*locked_tags_data, pooled_conn.mpd_conn())
                .await;
        }

        // Main MPD queue management - use pooled connection
        let mpd_queue_result = pooled_conn.mpd_conn().mpd.queue();

        match mpd_queue_result {
            Ok(queue) => {
                let now_playing_len = queue.len();
                if now_playing_len < 2 {
                    if let Some(song) = locked_song_queue.remove() {
                        if let Err(error) = pooled_conn.mpd_conn().mpd.push(song.clone()) {
                            eprintln!("[!] Error pushing song to MPD: {}", error);
                        } else {
                            info!("[+] scheduler adding song {}", song.file);
                            let _ = pooled_conn.mpd_conn().mpd.play();

                            // Request background precompute if queue is getting low
                            if locked_song_queue.len() < 50 {
                                let album_aware = *app_state.album_aware.read().await;
                                locked_song_queue.request_precompute(album_aware);
                                info!("[+] Queue low, requested background precompute (album_aware: {})", album_aware);
                            }
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("[!] Error getting MPD queue: {}", error);
                // With pool, we don't need to reconnect manually - just let the connection drop
                // and get a fresh one next iteration
            }
        }

        // Print cache stats periodically
        if scheduler_cycle % 100 == 0 {
            // Every 5 minutes
            let (hits, misses, hit_rate) = locked_song_queue.cache_stats();
            let (regular_valid, album_valid) =
                locked_song_queue.has_valid_cache(&*locked_tags_data);
            info!(
                "[+] Cache stats - Hits: {}, Misses: {}, Hit rate: {:.1}%",
                hits, misses, hit_rate
            );
            info!(
                "[+] Cache validity - Regular: {}, Album: {}",
                regular_valid, album_valid
            );
        }

        // No need to explicitly drop - pooled_conn will return to pool automatically
        drop(locked_song_queue);
        drop(locked_tags_data);

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn background_precompute_task(app_state: AppState) {
    info!("[+] Background precompute task started");

    // Wait a bit before starting to let the main system initialize
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mut cycle = 0u64;

    loop {
        cycle += 1;

        // Run background precompute every 30 seconds
        tokio::time::sleep(Duration::from_secs(30)).await;

        warn!("[+] Background precompute cycle #{}", cycle);

        // Get connection from pool
        let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!(
                    "[!] Error getting MPD connection from pool in background task: {}",
                    e
                );
                continue;
            }
        };

        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // Perform background precomputation
        let precompute_start = std::time::Instant::now();
        locked_song_queue
            .background_precompute(&*locked_tags_data, pooled_conn.mpd_conn())
            .await;

        let precompute_time = precompute_start.elapsed();
        if precompute_time.as_millis() > 100 {
            // Only log if it took significant time
            info!(
                "[+] Background precompute completed in {:?}",
                precompute_time
            );
        }

        drop(locked_song_queue);
        drop(locked_tags_data);
        // pooled_conn returns to pool automatically

        // TODO: wtf this feels entirely bunk
        //// More frequent precompute if we're in album-aware mode (it's more expensive)
        //let album_aware = *app_state.album_aware.read().await;
        //if album_aware && cycle % 2 == 0 {
        //    // Extra precompute every minute for album mode
        //    tokio::time::sleep(Duration::from_secs(30)).await;

        //    // Get another connection from pool for the extra precompute
        //    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        //        Ok(conn) => conn,
        //        Err(e) => {
        //            eprintln!(
        //                "[!] Error getting MPD connection for extra precompute: {}",
        //                e
        //            );
        //            continue;
        //        }
        //    };

        //    let mut locked_song_queue = app_state.song_queue.write().await;
        //    let locked_tags_data = app_state.tags_data.read().await;

        //    println!("[+] Extra album-aware precompute cycle");
        //    locked_song_queue
        //        .background_precompute(&*locked_tags_data, pooled_conn.mpd_conn())
        //        .await;

        //    drop(locked_song_queue);
        //    drop(locked_tags_data);
        //    // pooled_conn returns to pool automatically
        //}
    }
}
