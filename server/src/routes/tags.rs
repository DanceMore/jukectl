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
    pub album_aware: Option<bool>,
    pub album_tags: Option<Vec<String>>,
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
    if let Some(album_aware) = tags_update.album_aware {
        locked_tags_data.album_aware = album_aware;
        locked_song_queue.set_album_aware(album_aware);
        println!("[+] album-aware mode set to: {}", album_aware);
    }
    if let Some(album_tags) = &tags_update.album_tags {
        locked_tags_data.album_tags = album_tags.clone();
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

// New route specifically for toggling album-aware mode
#[post("/album-mode/<enabled>")]
async fn set_album_mode(
    enabled: bool,
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let mut locked_tags_data = app_state.tags_data.write().await;

    // Update the album-aware setting
    locked_tags_data.album_aware = enabled;
    locked_song_queue.set_album_aware(enabled);
    
    println!("[+] album-aware mode toggled to: {}", enabled);
    
    // Regenerate the queue with the new mode
    let songs = locked_tags_data.get_allowed_songs(&mut locked_mpd_conn);
    locked_song_queue.shuffle_and_add(songs);

    let response = serde_json::json!({
        "album_aware": enabled,
        "message": if enabled { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    });

    Json(response)
}

// Return routes defined in this module
pub fn routes() -> Vec<rocket::Route> {
    routes![tags, update_tags, set_album_mode]
}
