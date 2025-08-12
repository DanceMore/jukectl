// index.rs - Updated to use connection pool
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
    println!("[-] inside index method");
    
    // Get connection from pool instead of locking single connection
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json(Vec::new());
        }
    };

    // Attempt to retrieve the song queue
    let song_array = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(queue) => queue,
        Err(error) => {
            eprintln!("[!] Error retrieving song queue: {}", error);
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
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json(SkipResponse {
                skipped: String::new(),
                new: String::new(),
            });
        }
    };

    // Get the first song from the now playing queue
    let now_playing_queue = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(queue) => queue,
        Err(e) => {
            eprintln!("[!] Failed to get MPD queue: {}", e);
            return Json(SkipResponse {
                skipped: String::new(),
                new: String::new(),
            });
        }
    };

    let skipped_song = now_playing_queue
        .first()
        .map(|song| song.file.clone())
        .unwrap_or_default();
    let new_song = now_playing_queue
        .get(1)
        .map(|song| song.file.clone())
        .unwrap_or_default();

    // Delete the first song (skip)
    if let Err(e) = pooled_conn.mpd_conn().mpd.delete(0) {
        eprintln!("[!] Error skipping song: {}", e);
    }

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
