#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use rocket::tokio::time::Duration;
use serde::Deserialize;
use serde::Serialize;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

mod models;
use crate::models::hashable_song::HashableSong;
use crate::models::song_queue::SongQueue;
use crate::models::tags_data::TagsData;

mod mpd_conn;
use crate::mpd_conn::MpdConn;

fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    let mut filename_array = Vec::new();

    for song in song_array {
        filename_array.push(song.file);
    }

    filename_array
}

// TODO: move out of main.rs
fn scheduler_mainbody(
    mpd_conn: Arc<Mutex<MpdConn>>,
    song_queue: Arc<Mutex<SongQueue>>,
    tags_data: Arc<Mutex<TagsData>>,
) {
    loop {
        debug!("[-] scheduler firing");
        // get locks
        let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");
        let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
        let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");

        // make sure SongQueue is not empty
        if locked_song_queue.len() == 0 {
            info!("[!] scheduler sees an empty queue, refilling...");
            // If song_queue is empty, fetch songs and add them
            let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
            locked_song_queue.shuffle_and_add(songs);
        }

        // only do work if the live MPD queue length is less than 2
        // ie: 1 Song now-playing, 1 Song on-deck
        let now_playing_len = locked_mpd_conn
            .mpd
            .queue()
            .expect("Failed getting MPD active-queue")
            .len();

        if now_playing_len < 2 {
            if let Some(song) = locked_song_queue.remove() {
                if let Err(error) = locked_mpd_conn.mpd.push(song.clone()) {
                    // Handle the error here or propagate it up to the caller
                    // In this example, we're printing the error and continuing
                    eprintln!("Error pushing song to MPD: {}", error);
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

        // release our locks
        drop(locked_mpd_conn);
        drop(locked_song_queue);
        drop(locked_tags_data);

        // Sleep for a while
        thread::sleep(Duration::from_secs(3));
    }
}

//#[derive(Serialize)]
//struct IndexResponse {
//    #[serde(skip_serializing_if = "Vec::is_empty")]
//    songs: Vec<String>,
//}
#[get("/")]
fn index(mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>) -> Json<Vec<String>> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");

    // Attempt to retrieve the song queue
    let song_array = match locked_mpd_conn.mpd.queue() {
        Ok(queue) => queue,
        Err(error) => {
            eprintln!("Error retrieving song queue: {}", error);
            Vec::new()
        }
    };

    let res = queue_to_filenames(song_array);

    drop(locked_mpd_conn);

    Json(res)
}

#[derive(Serialize)]
struct SkipResponse {
    skipped: String,
    new: String,
}
#[post("/skip")]
fn skip(mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>) -> Json<SkipResponse> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");

    // Get the first song from the now playing queue
    let now_playing_queue = locked_mpd_conn
        .mpd
        .queue()
        .expect("Failed to get MPD queue");

    let skipped_song = now_playing_queue
        .get(0)
        .map(|song| song.file.clone())
        .unwrap_or_default();
    let new_song = now_playing_queue
        .get(1)
        .map(|song| song.file.clone())
        .unwrap_or_default();

    // Delete the first song (skip)
    // the API docs feel like I should be using mpd.next()
    // but that call seemed to do nothing ....? delete(0) is equivalent.
    let _res = locked_mpd_conn.mpd.delete(0);

    // all done, drop the lock ASAP
    drop(locked_mpd_conn);

    // Create the response struct with the data
    let response = SkipResponse {
        skipped: skipped_song,
        new: new_song,
    };

    Json(response)
}

#[get("/tags")]
fn tags(tags_data: &rocket::State<Arc<Mutex<TagsData>>>) -> Json<TagsData> {
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    Json(locked_tags_data.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsUpdate {
    pub any: Option<Vec<String>>,
    pub not: Option<Vec<String>>,
}
#[post("/tags", data = "<tags_update>")]
fn update_tags(
    tags_update: Json<TagsUpdate>,
    shared_tags_data: &rocket::State<Arc<Mutex<TagsData>>>,
    song_queue: &rocket::State<Arc<Mutex<SongQueue>>>,
    mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>,
) -> Json<TagsData> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let mut locked_tags_data = shared_tags_data.lock().expect("Failed to lock TagsData");

    // Check if 'any' and 'not' fields are present and update them if needed
    if let Some(any) = &tags_update.any {
        locked_tags_data.any = any.clone();
    }
    if let Some(not) = &tags_update.not {
        locked_tags_data.not = not.clone();
    }

    // If 'not' field is not empty, empty the 'TagsData.not' field
    if !tags_update
        .not
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        locked_tags_data.not.clear();
    }

    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
    locked_song_queue.shuffle_and_add(songs);

    let res = locked_tags_data.clone();

    // release our locks
    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    Json(res)
}

#[derive(Serialize)]
struct QueueResponse {
    length: usize,
    head: Vec<String>,
    tail: Vec<String>,
}

#[get("/queue")]
fn get_queue_length(song_queue: &rocket::State<Arc<Mutex<SongQueue>>>) -> Json<QueueResponse> {
    let locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let length = locked_song_queue.len(); // Get the length of the queue

    // TODO: I kinda hate this presentation layer formatting, but it compiles...
    let head = locked_song_queue
        .head()
        .iter()
        .map(|song| song.file.clone())
        .collect::<Vec<_>>();
    let tail = locked_song_queue
        .tail()
        .iter()
        .map(|song| song.file.clone())
        .collect::<Vec<_>>();

    let res = QueueResponse { length, head, tail };
    Json(res)
}

// TODO: implement JSON return struct, maybe rename to reload to match old API ..?
// POST /shuffle is new in Rust
// Ruby used POST /reload
//
//    old_pls = arr[0..3]
//    new_pls = arr[0..3]
//    res_old = queue_to_filenames(old_pls)
//    res_new = queue_to_filenames(new_pls)
//    json({:old => res_old, :new => res_new})

#[derive(rocket::serde::Serialize)]
struct ShuffleResponse {
    old: Vec<String>,
    new: Vec<String>,
}

#[post("/shuffle")]
fn shuffle_songs(
    song_queue: &rocket::State<Arc<Mutex<SongQueue>>>,
    tags_data: &rocket::State<Arc<Mutex<TagsData>>>,
    mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>,
) -> Json<ShuffleResponse> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");

    // Get the desired songs
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);

    // Handle the case where there are no valid songs
    if songs.is_empty() {
        return Json(ShuffleResponse {
            old: Vec::new(),
            new: Vec::new(),
        });
    }

    // Capture the old songs before shuffling
    let old_songs = (*locked_song_queue).head().clone();

    // Use a method on the SongQueue object to handle the shuffle and adding of songs
    locked_song_queue.shuffle_and_add(songs);

    // Capture the new songs after shuffling
    let new_songs = (*locked_song_queue).head().clone();

    let response = ShuffleResponse {
        old: queue_to_filenames(old_songs),
        new: queue_to_filenames(new_songs),
    };

    // release locks
    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    Json(response)
}

#[derive(serde::Deserialize)]
struct SongTagsUpdate {
    add: Vec<String>,
    remove: Vec<String>,
}

#[post("/song/tags", data = "<song_tags>")]
fn update_song_tags(
    song_tags: Json<SongTagsUpdate>,
    mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>,
) -> Json<String> {
    let add_tags = &song_tags.add; // Tags to add
    let remove_tags = &song_tags.remove; // Tags to remove

    // Lock the MPD client connection
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");

    // Get the first song from the now playing queue
    let first_song = locked_mpd_conn
        .mpd
        .queue()
        .expect("Failed to get MPD queue")
        .first()
        .cloned(); // Clone the song to work with

    if let Some(song) = first_song {
        for tag in add_tags {
            println!("[!] Add song to tag {}", tag);
            // Add the song to the playlist with the specified tag
            if let Err(error) = locked_mpd_conn.mpd.pl_push(tag, song.clone()) {
                eprintln!("Error adding song to tag playlist: {}", error);
            }
        }

        for tag in remove_tags {
            println!("[!] Remove song from tag {}", tag);

            // Find the song's position in the playlist with the specified tag
            let playlist = locked_mpd_conn.mpd.playlist(tag);
            if let Ok(playlist) = playlist {
                if let Some(position) = playlist
                    .iter()
                    .position(|song_to_remove| song_to_remove.file == song.file)
                {
                    // Delete the song at the found position
                    if let Err(error) = locked_mpd_conn.mpd.pl_delete(tag, position as u32) {
                        eprintln!("Error removing song from tag playlist: {}", error);
                    }
                } else {
                    println!("Song not found in the playlist with tag {}", tag);
                }
            } else {
                eprintln!("Error getting playlist: {}", playlist.err().unwrap());
            }
        }
    }

    Json("Tags updated successfully".to_string())
}

#[launch]
fn rocket() -> _ {
    // share the MpdConn and SongQueue
    let mpd_conn = Arc::new(Mutex::new(
        MpdConn::new().expect("Failed to create MPD connection"),
    ));
    let song_queue = Arc::new(Mutex::new(SongQueue::new()));

    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(Mutex::new(default_tags_data));

    // acquire locks for initial setup...
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");

    // set up the jukebox SongQueue at boot...
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
    locked_song_queue.shuffle_and_add(songs);

    // release locks
    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    // build some accessors for our Scheduler...
    let mpd_conn_clone = Arc::clone(&mpd_conn);
    let song_queue_clone = Arc::clone(&song_queue);
    let tags_data_clone = Arc::clone(&tags_data);

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    thread::spawn(|| scheduler_mainbody(mpd_conn_clone, song_queue_clone, tags_data_clone));

    rocket::build()
        .manage(tags_data) // Pass TagsData as a rocket::State
        .manage(song_queue) // Pass SongQueue as a rocket::State
        .manage(mpd_conn)
        .mount(
            "/",
            routes![
                index,
                tags,
                update_tags,
                get_queue_length,
                shuffle_songs,
                skip,
                update_song_tags
            ],
        )
}