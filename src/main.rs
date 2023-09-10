#[macro_use]
extern crate rocket;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::tokio::time::{interval, Duration};
use rocket::{Rocket, State};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};
use std::thread;

use std::collections::HashSet;

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

use std::io::Write;

fn scheduler_mainbody(song_queue: Arc<Mutex<SongQueue>>) {
    loop {
        info!("[-] scheduler firing");

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
                locked_mpd_conn.mpd.push(song);
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
fn index(mpd_conn: &State<Arc<Mutex<MpdConn>>>) -> Json<Vec<String>> {
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MPD connection");
    let mut song_array = locked_mpd_conn.mpd.queue().unwrap();

    let res = queue_to_filenames(song_array);

    Json(res)
}

#[get("/tags")]
fn tags(tags_data: &State<Arc<Mutex<TagsData>>>) -> Json<TagsData> {
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    Json(locked_tags_data.clone())
}

#[post("/tags", data = "<tags_data>")]
fn update_tags(
    tags_data: Json<TagsData>,
    shared_tags_data: &State<Arc<Mutex<TagsData>>>,
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
fn get_queue_length(song_queue: &State<Arc<Mutex<SongQueue>>>) -> Json<QueueResponse> {
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
    song_queue: &State<Arc<Mutex<SongQueue>>>,
    tags_data: &State<Arc<Mutex<TagsData>>>,
    mpd_conn: &State<Arc<Mutex<MpdConn>>>,
) -> Json<String> {
    let mut locked_song_queue = song_queue.lock().expect("Failed to lock SongQueue");
    let locked_tags_data = tags_data.lock().expect("Failed to lock TagsData");
    let mut locked_mpd_conn = mpd_conn.lock().expect("Failed to lock MpdConn");

    let tags = &locked_tags_data.any;

    // Get the desired songs
    let songs = get_allowed_songs(&locked_tags_data, &mut locked_mpd_conn);

    // Handle the case where there are no valid songs
    if songs.is_empty() {
        return Json("No valid songs to play. Bad human! No cookie!".to_string());
    }

    // Add the shuffled songs to the queue
    locked_song_queue.empty_queue();
    for song in songs {
        locked_song_queue.add(mpd::Song::from(song));
    }
    locked_song_queue.shuffle();

    Json("Songs shuffled and added to the queue.".to_string())
}

fn extract_tags(tags_data: &TagsData) -> (HashSet<String>, HashSet<String>) {
    let any_tags: HashSet<String> = tags_data
        .any
        .iter()
        .flat_map(|s| s.split(',').map(String::from))
        .collect();
    let not_tags: HashSet<String> = tags_data
        .not
        .iter()
        .flat_map(|s| s.split(',').map(String::from))
        .collect();

    (any_tags, not_tags)
}

fn get_allowed_songs(tags_data: &TagsData, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
    let (any_tags, not_tags) = extract_tags(tags_data);

    // Create a HashSet to store the desired songs
    let mut desired_songs = HashSet::new();

    // Process "any" tags
    for tag in &any_tags {
        if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
            for song in playlist {
                desired_songs.insert(HashableSong(song));
            }
        }
    }

    // Process "not" tags
    for tag in &not_tags {
        if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
            for song in playlist {
                desired_songs.remove(&HashableSong(song));
            }
        }
    }

    //println!("{:?}", desired_songs);

    desired_songs
}

fn init_mpd_conn() -> Arc<Mutex<MpdConn>> {
    let mpd_conn = MpdConn::new().expect("Failed to create MPD connection");
    Arc::new(Mutex::new(mpd_conn))
}

#[launch]
fn rocket() -> _ {
    // Shareable TagsData with default values
    let default_tags_data = TagsData {
        any: vec!["jukebox".to_string()],
        not: vec!["explicit".to_string()],
    };
    let tags_data = Arc::new(Mutex::new(default_tags_data));
    let song_queue = Arc::new(Mutex::new(SongQueue::new()));

    // build some accessors for our Scheduler...
    let song_queue_clone = Arc::clone(&song_queue);

    // Spawn a detached asynchronous task to run the scheduler_mainbody function
    thread::spawn(|| scheduler_mainbody(song_queue_clone));

    rocket::build()
        .manage(tags_data) // Pass TagsData as a state
        .manage(song_queue) // Pass SongQueue as a state
        .manage(init_mpd_conn())
        .mount(
            "/",
            routes![index, tags, update_tags, get_queue_length, shuffle_songs],
        )
}
