use crate::models::song_queue::SongQueue;
use crate::models::tags_data::TagsData;
use crate::mpd_conn::MpdConn;

use rocket::tokio::time::Duration;
use std::io::Write;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

pub fn scheduler_mainbody(song_queue: Arc<Mutex<SongQueue>>, tags_data: Arc<Mutex<TagsData>>) {
    // Create a new MpdConn instance for the scheduler
    let mpd_conn = Arc::new(Mutex::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));

    loop {
        debug!("[-] scheduler firing");
        // lock the local connector, should not matter
        let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");

        // shared data, these locks matter
        let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
        let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");

        // make sure SongQueue is not empty
        if locked_song_queue.len() == 0 {
            info!("[!] scheduler sees an empty queue, refilling...");
            // If song_queue is empty, fetch songs and add them
            let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
            locked_song_queue.shuffle_and_add(songs);
        }

        // aggressively release locks
        drop(locked_song_queue);
        drop(locked_tags_data);

        // only do work if the live MPD queue length is less than 2
        // ie: 1 Song now-playing, 1 Song on-deck
        let now_playing_len = locked_mpd_conn
            .mpd
            .queue()
            .expect("Failed getting MPD active-queue")
            .len();

        let mut locked_song_queue2 = song_queue.lock().expect("Failed to lock SongQueue");
        if now_playing_len < 2 {
            if let Some(song) = locked_song_queue2.remove() {
                if let Err(error) = locked_mpd_conn.mpd.push(song.clone()) {
                    // Handle the error here or propagate it up to the caller
                    // In this example, we're printing the error and continuing
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

        // release locks
        drop(locked_song_queue2);

        // Sleep for a while
        thread::sleep(Duration::from_secs(3));
    }
}
