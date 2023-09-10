#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use rocket::tokio::time::Duration;
use serde::Serialize;

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::thread;

mod models;
use crate::models::hashable_song::HashableSong;
use crate::models::tags_data::TagsData;

mod mpd_conn;
use crate::mpd_conn::MpdConn;

mod song_queue;
use crate::song_queue::SongQueue;

fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    let mut filename_array = Vec::new();

    for song in song_array {
        filename_array.push(song.file);
    }

    filename_array
}

fn scheduler_mainbody(song_queue: Arc<Mutex<SongQueue>>) {
    loop {
        debug!("[-] scheduler firing");

        // Access song_queue by locking the mutex
        let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");

        // lock mpd conn
        let mpd_conn = init_mpd_conn();
        let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");

        let now_playing_len = locked_mpd_conn.mpd.queue().expect("REASON").len();

        // only do work if the live MPD queue length is less than 2
        // ie: 1 Song now-playing, 1 Song on-deck
        if now_playing_len < 2 {
            if let Some(song) = locked_song_queue.remove() {
                if let Err(error) = locked_mpd_conn.mpd.push(song.clone()) {
                    // Handle the error here or propagate it up to the caller
                    // In this example, we're printing the error and continuing
                    eprintln!("Error pushing song to MPD: {}", error);
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

        // Sleep for a while
        thread::sleep(Duration::from_secs(3));
    }
}

#[get("/")]
fn index(mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>) -> Json<Vec<String>> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");
    let song_array = locked_mpd_conn.mpd.queue().unwrap();

    let res = queue_to_filenames(song_array);

    Json(res)
}

#[get("/tags")]
fn tags(tags_data: &rocket::State<Arc<Mutex<TagsData>>>) -> Json<TagsData> {
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    Json(locked_tags_data.clone())
}

#[post("/tags", data = "<tags_data>")]
fn update_tags(
    tags_data: Json<TagsData>,
    shared_tags_data: &rocket::State<Arc<Mutex<TagsData>>>,
) -> Json<TagsData> {
    let mut locked_data = shared_tags_data.lock().expect("Failed to lock TagsData");
    *locked_data = tags_data.0.clone();
    Json(locked_data.clone())
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

#[post("/shuffle")]
fn shuffle_songs(
    song_queue: &rocket::State<Arc<Mutex<SongQueue>>>,
    tags_data: &rocket::State<Arc<Mutex<TagsData>>>,
    mpd_conn: &rocket::State<Arc<Mutex<MpdConn>>>,
) -> Json<String> {
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");

    // Get the desired songs
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);

    // Handle the case where there are no valid songs
    if songs.is_empty() {
        return Json("No valid songs to play. Bad human! No cookie!".to_string());
    }

    // Use a method on the SongQueue object to handle the shuffle and adding of songs
    locked_song_queue.shuffle_and_add(songs);

    Json("Songs shuffled and added to the queue.".to_string())
}

// the Arc/Mutex is here ....
fn init_mpd_conn() -> Arc<Mutex<MpdConn>> {
    let mpd_conn = MpdConn::new().expect("Failed to create MPD connection");
    Arc::new(Mutex::new(mpd_conn))
}

#[launch]
fn rocket() -> _ {
    let mpd_conn = init_mpd_conn();

    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(Mutex::new(default_tags_data));

    let song_queue = Arc::new(Mutex::new(SongQueue::new()));

    // acquire locks for initial setup...
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");

    // set up the jukebox SongQueue at boot...
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
    locked_song_queue.shuffle_and_add(songs);

    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    // build some accessors for our Scheduler...
    let song_queue_clone = Arc::clone(&song_queue);

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    thread::spawn(|| scheduler_mainbody(song_queue_clone));

    rocket::build()
        .manage(tags_data) // Pass TagsData as a rocket::State
        .manage(song_queue) // Pass SongQueue as a rocket::State
        .manage(init_mpd_conn())
        .mount(
            "/",
            routes![index, tags, update_tags, get_queue_length, shuffle_songs],
        )
}
