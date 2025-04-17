use rocket::serde::json::Json;
use rocket::Route;
use rocket::State;

use crate::app_state::AppState;

// Helper function moved from main.rs
fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    song_array.into_iter().map(|song| song.file).collect()
}

#[get("/")]
pub async fn index(app_state: &State<AppState>) -> Json<Vec<String>> {
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

    Json(res)
}

#[derive(serde::Serialize)]
pub struct SkipResponse {
    skipped: String,
    new: String,
}

#[post("/skip")]
pub async fn skip(app_state: &State<AppState>) -> Json<SkipResponse> {
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
    let _res = locked_mpd_conn.mpd.delete(0);

    // Create the response struct with the data
    let response = SkipResponse {
        skipped: skipped_song,
        new: new_song,
    };

    Json(response)
}

// Return routes defined in this module
pub fn routes() -> Vec<Route> {
    routes![index, skip]
}
