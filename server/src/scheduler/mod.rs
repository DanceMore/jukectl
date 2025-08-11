use rocket::tokio;
use rocket::tokio::time::Duration;
use std::io::Write;

use crate::app_state::AppState;

pub async fn start_scheduler(app_state: AppState) {
    println!("[+] Starting enhanced scheduler with background precomputation...");

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
            println!("[-] scheduler cycle #{}", scheduler_cycle);
        } else {
            print!(".");
            let _ = std::io::stdout().flush();
        }

        let mut locked_mpd_conn = app_state.mpd_conn.write().await;
        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // Check if SongQueue is empty and refill if needed
        if locked_song_queue.len() == 0 {
            println!("[!] scheduler sees an empty queue, refilling with cached data...");

            // Use the ultra-fast async method for instant refills
            locked_song_queue
                .shuffle_and_add_with_cache_async(&*locked_tags_data, &mut *locked_mpd_conn)
                .await;
        }

        // Main MPD queue management (unchanged)
        let mpd_queue_result = locked_mpd_conn.mpd.queue();

        match mpd_queue_result {
            Ok(queue) => {
                let now_playing_len = queue.len();
                if now_playing_len < 2 {
                    if let Some(song) = locked_song_queue.remove() {
                        if let Err(error) = locked_mpd_conn.mpd.push(song.clone()) {
                            eprintln!("[!] Error pushing song to MPD: {}", error);
                        } else {
                            println!("[+] scheduler adding song {}", song.file);
                            let _ = locked_mpd_conn.mpd.play();

                            // Request background precompute if queue is getting low
                            if locked_song_queue.len() < 50 {
                                let album_aware = *app_state.album_aware.read().await;
                                locked_song_queue.request_precompute(album_aware);
                                println!("[+] Queue low, requested background precompute (album_aware: {})", album_aware);
                            }
                        }
                    }
                }
            }
            Err(error) => {
                eprintln!("[!] Error getting MPD queue: {}", error);
                if let Err(reconnect_error) = locked_mpd_conn.reconnect() {
                    eprintln!("[!] Error reconnecting to MPD: {}", reconnect_error);
                }
            }
        }

        // Print cache stats periodically
        if scheduler_cycle % 100 == 0 {
            // Every 5 minutes
            let (hits, misses, hit_rate) = locked_song_queue.cache_stats();
            let (regular_valid, album_valid) =
                locked_song_queue.has_valid_cache(&*locked_tags_data);
            println!(
                "[+] Cache stats - Hits: {}, Misses: {}, Hit rate: {:.1}%",
                hits, misses, hit_rate
            );
            println!(
                "[+] Cache validity - Regular: {}, Album: {}",
                regular_valid, album_valid
            );
        }

        drop(locked_song_queue);
        drop(locked_tags_data);
        drop(locked_mpd_conn);

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn background_precompute_task(app_state: AppState) {
    println!("[+] Background precompute task started");

    // Wait a bit before starting to let the main system initialize
    tokio::time::sleep(Duration::from_secs(10)).await;

    let mut cycle = 0u64;

    loop {
        cycle += 1;

        // Run background precompute every 30 seconds
        tokio::time::sleep(Duration::from_secs(30)).await;

        println!("[+] Background precompute cycle #{}", cycle);

        // Get locks in the right order to avoid deadlocks
        let mut locked_mpd_conn = app_state.mpd_conn.write().await;
        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // Perform background precomputation
        let precompute_start = std::time::Instant::now();
        locked_song_queue
            .background_precompute(&*locked_tags_data, &mut *locked_mpd_conn)
            .await;

        let precompute_time = precompute_start.elapsed();
        if precompute_time.as_millis() > 100 {
            // Only log if it took significant time
            println!(
                "[+] Background precompute completed in {:?}",
                precompute_time
            );
        }

        drop(locked_song_queue);
        drop(locked_tags_data);
        drop(locked_mpd_conn);

        // More frequent precompute if we're in album-aware mode (it's more expensive)
        let album_aware = *app_state.album_aware.read().await;
        if album_aware && cycle % 2 == 0 {
            // Extra precompute every minute for album mode
            tokio::time::sleep(Duration::from_secs(30)).await;

            let mut locked_mpd_conn = app_state.mpd_conn.write().await;
            let mut locked_song_queue = app_state.song_queue.write().await;
            let locked_tags_data = app_state.tags_data.read().await;

            println!("[+] Extra album-aware precompute cycle");
            locked_song_queue
                .background_precompute(&*locked_tags_data, &mut *locked_mpd_conn)
                .await;

            drop(locked_song_queue);
            drop(locked_tags_data);
            drop(locked_mpd_conn);
        }
    }
}
