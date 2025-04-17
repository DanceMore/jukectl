use rocket::tokio;
use rocket::tokio::time::Duration;
use std::io::Write;

use crate::app_state::AppState;

pub async fn start_scheduler(app_state: AppState) {
    println!("[+] Starting scheduler...");
    tokio::spawn(scheduler_mainbody(app_state));
}

async fn scheduler_mainbody(app_state: AppState) {
    loop {
        debug!("[-] scheduler firing");
        
        // Get locks asynchronously
        let mut locked_mpd_conn = app_state.mpd_conn.write().await;
        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // make sure SongQueue is not empty
        if locked_song_queue.len() == 0 {
            info!("[!] scheduler sees an empty queue, refilling...");
            // If song_queue is empty, fetch songs and add them
            let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
            locked_song_queue.shuffle_and_add(songs);
        }

        // only do work if the live MPD queue length is less than 2
        // ie: 1 Song now-playing, 1 Song on-deck
        let mpd_queue_result = locked_mpd_conn.mpd.queue();
        
        match mpd_queue_result {
            Ok(queue) => {
                let now_playing_len = queue.len();
                if now_playing_len < 2 {
                    if let Some(song) = locked_song_queue.remove() {
                        if let Err(error) = locked_mpd_conn.mpd.push(song.clone()) {
                            eprintln!("[!] Error pushing song to MPD: {}", error);
                        } else {
                            info!("[+] scheduler adding song {}", song.file);
                            let _ = locked_mpd_conn.mpd.play();
                        }
                    }
                } else {
                    // do nothing, but let's print to prove we worked...
                    print!(".");
                    let _ = std::io::stdout().flush();
                }
            },
            Err(error) => {
                eprintln!("[!] Error getting MPD queue: {}", error);
                // Consider reconnecting here
                if let Err(reconnect_error) = locked_mpd_conn.reconnect() {
                    eprintln!("[!] Error reconnecting to MPD: {}", reconnect_error);
                }
            }
        }

        // Locks are automatically released when they go out of scope
        
        // Non-blocking sleep using tokio
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
