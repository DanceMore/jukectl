use rocket::serde::json::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::app_state::AppState;
use jukectl_server::models::tags_data::TagsData;

#[get("/tags")]
async fn tags(app_state: &rocket::State<AppState>) -> Json<TagsData> {
    let read_guard = app_state.tags_data.read().await;
    Json(read_guard.clone())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsUpdate {
    pub any: Option<Vec<String>>,
    pub not: Option<Vec<String>>,
    // album_aware and album_tags removed as they are now in AppState
}

#[post("/tags", data = "<tags_update>")]
async fn update_tags(
    tags_update: Json<TagsUpdate>,
    app_state: &rocket::State<AppState>,
) -> Json<TagsData> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let mut locked_tags_data = app_state.tags_data.write().await;
    let locked_album_aware = app_state.album_aware.read().await;

    // Check if 'any' and 'not' fields are present and update them if needed
    if let Some(any) = &tags_update.any {
        locked_tags_data.any = any.clone();
    }
    if let Some(not) = &tags_update.not {
        locked_tags_data.not = not.clone();
    }
    // Album-aware mode is now handled by AppState, not TagsData

    // If 'not' field is not empty, empty the 'TagsData.not' field
    if !tags_update
        .not
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        locked_tags_data.not.clear();
    }

    let songs = if *locked_album_aware {
        locked_tags_data.get_album_aware_songs(&mut locked_mpd_conn)
    } else {
        locked_tags_data.get_allowed_songs(&mut locked_mpd_conn)
    };
    locked_song_queue.shuffle_and_add(songs);

    let res = locked_tags_data.clone();

    // release our locks
    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    Json(res)
}

// New route specifically for toggling album-aware mode
#[post("/album-mode/<enabled>")]
async fn set_album_mode(
    enabled: bool,
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    // Update the album-aware setting in AppState
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;

    // TODO: this probably does need to be mutable, why aren't we updating it? we only update song_queue ???
    let mut locked_album_aware = app_state.album_aware.write().await;

    // Update the song queue with new settings
    locked_song_queue.set_album_aware(enabled);

    println!("[+] album-aware mode set to: {}", enabled);

    // Regenerate the queue with the new mode
    let songs = if enabled {
        locked_tags_data.get_album_aware_songs(&mut locked_mpd_conn)
    } else {
        locked_tags_data.get_allowed_songs(&mut locked_mpd_conn)
    };
    locked_song_queue.shuffle_and_add(songs);

    let response = serde_json::json!({
        "album_aware": *locked_album_aware,
        "message": if *locked_album_aware { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    });

    Json(response)
}

// Toggle album-aware mode
#[post("/album-mode/toggle")]
async fn toggle_album_mode(
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    // Toggle the album-aware setting in AppState
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;

    // TODO: this probably DOES need to be mutable, why are we not toggling app_state.album_aware and only touching song_queue again
    let mut locked_album_aware = app_state.album_aware.write().await;

    // Update the song queue with new settings
    locked_song_queue.set_album_aware(*locked_album_aware);

    println!("[+] album-aware mode toggled to: {}", locked_album_aware);

    // Regenerate the queue with the new mode
    let songs = if *locked_album_aware {
        locked_tags_data.get_album_aware_songs(&mut locked_mpd_conn)
    } else {
        locked_tags_data.get_allowed_songs(&mut locked_mpd_conn)
    };
    locked_song_queue.shuffle_and_add(songs);

    let response = serde_json::json!({
        "album_aware": *locked_album_aware,
        "message": if *locked_album_aware { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    });

    Json(response)
}

// Return routes defined in this module
pub fn routes() -> Vec<rocket::Route> {
    routes![tags, update_tags, set_album_mode, toggle_album_mode]
}
