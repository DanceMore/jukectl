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

        let mut locked_mpd_conn = app_state.mpd_conn.write().await;
        let mut locked_song_queue = app_state.song_queue.write().await;
        let locked_tags_data = app_state.tags_data.read().await;

        // Make sure SongQueue is not empty - use caching for instant refills!
        if locked_song_queue.len() == 0 {
            info!("[!] scheduler sees an empty queue, refilling...");
            // This will be super fast due to caching
            locked_song_queue.shuffle_and_add_with_cache(&*locked_tags_data, &mut *locked_mpd_conn);
        }

        // Rest of scheduler logic stays the same...
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
                    print!(".");
                    let _ = std::io::stdout().flush();
                }
            }
            Err(error) => {
                eprintln!("[!] Error getting MPD queue: {}", error);
                if let Err(reconnect_error) = locked_mpd_conn.reconnect() {
                    eprintln!("[!] Error reconnecting to MPD: {}", reconnect_error);
                }
            }
        }

        drop(locked_song_queue);
        drop(locked_tags_data);
        drop(locked_mpd_conn);

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
