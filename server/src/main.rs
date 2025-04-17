#[macro_use]
extern crate rocket;

use rocket::serde::json::Json;
use serde::Deserialize;
use serde::Serialize;

// local imports
mod app_state;
use app_state::AppState;
mod scheduler;
use scheduler::start_scheduler;

use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::models::tags_data::TagsData;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;

fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    let mut filename_array = Vec::new();

    for song in song_array {
        filename_array.push(song.file);
    }

    filename_array
}

#[get("/")]
async fn index(app_state: &rocket::State<AppState>) -> Json<Vec<String>> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;

    println!("[-] inside index method");
    // Attempt to retrieve the song queue
    let song_array = match locked_mpd_conn.mpd.queue() {
        Ok(queue) => queue,
        Err(error) => {
            eprintln!(
                "[!] Error retrieving song queue, triggering reconnect?: {}",
                error
            );
            if let Err(reconnect_error) = locked_mpd_conn.reconnect() {
                eprintln!("[!] Error reconnecting to MPD: {}", reconnect_error);
            }
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
async fn skip(app_state: &rocket::State<AppState>) -> Json<SkipResponse> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;

    // Get the first song from the now playing queue
    let now_playing_queue = locked_mpd_conn
        .mpd
        .queue()
        .expect("Failed to get MPD queue");

    let skipped_song = now_playing_queue
        .first()
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
async fn tags(app_state: &rocket::State<AppState>) -> Json<TagsData> {
    let read_guard = app_state.tags_data.read().await;
    Json(read_guard.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsUpdate {
    pub any: Option<Vec<String>>,
    pub not: Option<Vec<String>>,
}

#[post("/tags", data = "<tags_update>")]
async fn update_tags(
    tags_update: Json<TagsUpdate>,
    app_state: &rocket::State<AppState>,
) -> Json<TagsData> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let mut locked_tags_data = app_state.tags_data.write().await;

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

// Your existing route handler for /queue.
#[get("/queue?<count>")]
async fn get_queue(
    app_state: &rocket::State<AppState>,
    count: Option<usize>,
) -> Json<QueueResponse> {
    let count_value = count.unwrap_or(3); // Use a default value of 3 if count is None

    let locked_song_queue = app_state.song_queue.read().await;
    let length = locked_song_queue.len(); // Get the length of the queue

    // TODO: I kinda hate this presentation layer formatting, but it compiles...
    let head = locked_song_queue
        .head(Some(count_value))
        .iter()
        .map(|song| song.file.clone())
        .collect::<Vec<_>>();
    let tail = locked_song_queue
        .tail(Some(count_value))
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
async fn shuffle_songs(app_state: &rocket::State<AppState>) -> Json<ShuffleResponse> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;

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
    let old_songs = (*locked_song_queue).head(None).clone();

    // Use a method on the SongQueue object to handle the shuffle and adding of songs
    locked_song_queue.shuffle_and_add(songs);

    // Capture the new songs after shuffling
    let new_songs = (*locked_song_queue).head(None).clone();

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
async fn update_song_tags(
    song_tags: Json<SongTagsUpdate>,
    app_state: &rocket::State<AppState>,
) -> Json<String> {
    let add_tags = &song_tags.add; // Tags to add
    let remove_tags = &song_tags.remove; // Tags to remove

    // Lock the MPD client connection
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;

    // Get the first song from the now playing queue
    let first_song = locked_mpd_conn
        .mpd
        .queue()
        .expect("Failed to get MPD queue")
        .first()
        .cloned(); // Clone the song to work with

    if let Some(song) = first_song {
        for tag in add_tags {
            println!("[+] Add song to tag {}", tag);
            // Add the song to the playlist with the specified tag
            if let Err(error) = locked_mpd_conn.mpd.pl_push(tag, song.clone()) {
                eprintln!("[!] Error adding song to tag playlist: {}", error);
            }
        }

        // Start the timer
        let remove_tags_timer = std::time::Instant::now();
        for tag in remove_tags {
            println!("[!] Remove song from tag {}", tag);

            let fetch_timer = std::time::Instant::now();
            // Find the song's position(s) in the playlist with the specified tag
            let playlist = match locked_mpd_conn.mpd.playlist(tag) {
                Ok(playlist) => playlist,
                Err(err) => {
                    eprintln!("[!] Error getting playlist: {}", err);
                    continue;
                }
            };
            let fetch_timer_elapsed = fetch_timer.elapsed();
            println!("[-] network fetch took: {:?}", fetch_timer_elapsed);

            let delete_timer = std::time::Instant::now();
            let positions_to_delete: Vec<_> = playlist
                .iter()
                .enumerate()
                .filter(|(_, song_to_remove)| song_to_remove.file == song.file)
                .map(|(position, _)| position as u32)
                .collect();
            let delete_timer_elapsed = delete_timer.elapsed();
            println!("[-] innermost walk took: {:?}", delete_timer_elapsed);

            // Delete the songs at the found positions
            for position in positions_to_delete.iter() {
                if let Err(error) = locked_mpd_conn.mpd.pl_delete(tag, *position) {
                    eprintln!("Error removing song from tag playlist: {}", error);
                }
            }

            // Print how many times the song was removed
            if positions_to_delete.is_empty() {
                println!("Song not found in the playlist with tag {}", tag);
            } else {
                println!(
                    "Song removed {} times from the playlist with tag {}",
                    positions_to_delete.len(),
                    tag
                );
            }
        }
        drop(locked_mpd_conn);

        // Stop the timer
        let elapsed_time = remove_tags_timer.elapsed();
        println!("[-] remove_tags took: {:?}", elapsed_time);
    }

    Json("Tags updated successfully".to_string())
}

#[launch]
fn rocket() -> _ {
    // Initialize the app state
    let app_state = app_state::initialize();

    // Build the rocket instance with routes and scheduler
    rocket::build()
        .manage(app_state.clone())
        .mount(
            "/",
            routes![
                index,
                tags,
                update_tags,
                get_queue,
                shuffle_songs,
                skip,
                update_song_tags
            ],
        )
        .attach(rocket::fairing::AdHoc::on_liftoff("Initialize Queue and Scheduler", |rocket| {
            Box::pin(async move {
                let state = rocket.state::<AppState>().unwrap();
                app_state::initialize_queue(state).await;
                start_scheduler(app_state).await;
            })
        }))
}
