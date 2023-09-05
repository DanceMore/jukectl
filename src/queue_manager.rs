use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex};
use rand::seq::SliceRandom;

use mpd::Song;
use mpd::Client;

// Define a custom error type
#[derive(Debug)]
struct QueueManagerError(String);

impl fmt::Display for QueueManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for QueueManagerError {}

//struct MPDConnection {
//    // Implement the MPDConnection struct as needed
//    // You can use the `mpd` crate to interact with MPD
//    let mut conn = Client::connect("nas.dance.more:6600").unwrap();
//}
//
//struct TagManager {
//    // Implement the TagManager struct as needed
//}
//
//struct QueueManager {
//    mpd: Arc<Mutex<MPDConnection>>,
//    tags: Arc<Mutex<TagManager>>,
//    queue: Vec<Song>,
//    songs: Vec<Song>,
//}
//
//impl QueueManager {
//    fn new(mpd_conn: Arc<Mutex<MPDConnection>>, tag_mgr: Arc<Mutex<TagManager>>) -> Result<Self, Box<dyn Error>> {
//        println!("[!] building QueueManager");
//
//        // Create a new song list and shuffle it
//        let queue = Self::shuffle(&tag_mgr)?;
//
//        Ok(QueueManager {
//            mpd: mpd_conn,
//            tags: tag_mgr,
//            queue,
//            songs: Vec::new(),
//        })
//    }
//
//    fn now_playing(&self) -> Option<Vec<Song>> {
//        let mpd_conn = self.mpd.lock().unwrap(); // Replace with proper MPD connection logic
//        let queue = mpd_conn.queue();
//
//        if queue.is_empty() {
//            None
//        } else {
//            Some(queue.iter().take(2).cloned().collect())
//        }
//    }
//
//    fn skip(&self) -> Result<(), Box<dyn Error>> {
//        let mut mpd_conn = self.mpd.lock().unwrap(); // Replace with proper MPD connection logic
//        mpd_conn.next()?;
//        Ok(())
//    }
//
//    fn add_song(&mut self) -> Result<bool, Box<dyn Error>> {
//        let mut mpd_conn = self.mpd.lock().unwrap(); // Replace with proper MPD connection logic
//        if mpd_conn.queue().len() < 2 {
//            self.add_random_song(&mut mpd_conn)?;
//            Ok(true)
//        } else {
//            Ok(false)
//        }
//    }
//
//    fn shuffle(&mut self) -> Result<(), Box<dyn Error>> {
//        let tags = self.tags.lock().unwrap(); // Replace with proper TagManager logic
//
//        // Grab our jukebox songs
//        let mut songs_any = self.get_songs_by_tags(&tags.tags["any"])?;
//        if let Some(not_tags) = &tags.tags["not"] {
//            let not_songs = self.get_songs_by_tags(not_tags)?;
//            songs_any.retain(|song| !not_songs.contains(song));
//        }
//
//        if songs_any.is_empty() {
//            return Err(Box::new(QueueManagerError("[!!!] no valid songs to play. bad human! no cookie!".to_string())));
//        }
//
//        // Finalize the data for usage
//        self.songs = songs_any;
//        self.songs.shuffle(&mut rand::thread_rng()); // Import rand crate for shuffling
//
//        Ok(())
//    }
//
//    fn get_songs_by_tags(&self, tags: &[String]) -> Result<Vec<Song>, Box<dyn Error>> {
//        let mpd_conn = self.mpd.lock().unwrap(); // Replace with proper MPD connection logic
//
//        let mut songs = Vec::new();
//
//        for tag in tags {
//            if let Some(playlist) = mpd_conn.playlists.iter().find(|s| s.name == *tag) {
//                songs.extend_from_slice(&playlist.songs);
//            }
//        }
//
//        Ok(songs)
//    }
//
//    fn add_random_song(&mut self, mpd_conn: &mut MPDConnection) -> Result<(), Box<dyn Error>> {
//        let song = self.songs.pop();
//
//        let song = if let Some(song) = song {
//            song
//        } else {
//            self.shuffle()?;
//            self.songs.pop().ok_or(QueueManagerError("no songs to add".to_string()))?
//        };
//
//        mpd_conn.add_song(&song.file)?;
//        mpd_conn.play()?;
//        Ok(())
//    }
//}
