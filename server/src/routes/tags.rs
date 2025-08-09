use rocket::serde::json::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::app_state::AppState;
use jukectl_server::models::tags_data::TagsData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsResponse {
    pub any: Vec<String>,
    pub not: Vec<String>,
    pub album_aware: bool,
}

#[get("/tags")]
async fn tags(app_state: &rocket::State<AppState>) -> Json<TagsResponse> {
    let read_guard = app_state.tags_data.read().await;
    let album_aware = *app_state.album_aware.read().await;

    Json(TagsResponse {
        any: read_guard.any.clone(),
        not: read_guard.not.clone(),
        album_aware,
    })
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

    // Check if 'any' and 'not' fields are present and update them if needed
    if let Some(any) = &tags_update.any {
        locked_tags_data.any = any.clone();
        // Tags changed, invalidate cache for fresh results
        locked_song_queue.invalidate_cache();
    }
    if let Some(not) = &tags_update.not {
        locked_tags_data.not = not.clone();
        // Tags changed, invalidate cache for fresh results  
        locked_song_queue.invalidate_cache();
    }

    // If 'not' field is not empty, empty the 'TagsData.not' field
    if !tags_update
        .not
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        locked_tags_data.not.clear();
        locked_song_queue.invalidate_cache();
    }

    // Use the new caching method for excellent performance
    locked_song_queue.shuffle_and_add_with_cache(&*locked_tags_data, &mut *locked_mpd_conn);

    let res = locked_tags_data.clone();
    drop(locked_mpd_conn);
    drop(locked_song_queue);
    drop(locked_tags_data);

    Json(res)
}

#[post("/album-mode/<enabled>")]
async fn set_album_mode(
    enabled: bool,
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;
    let mut locked_album_aware = app_state.album_aware.write().await;

    *locked_album_aware = enabled;
    
    // This will invalidate cache if mode actually changed
    locked_song_queue.set_album_aware(enabled);
    
    println!("[+] album-aware mode set to: {}", enabled);

    // Use caching for instant response (or very fast cache refresh)
    locked_song_queue.shuffle_and_add_with_cache(&*locked_tags_data, &mut *locked_mpd_conn);

    let response = serde_json::json!({
        "album_aware": enabled,
        "message": if enabled { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    });

    Json(response)
}

#[post("/album-mode/toggle")]
async fn toggle_album_mode(
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    let mut locked_mpd_conn = app_state.mpd_conn.write().await;
    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;
    let mut locked_album_aware = app_state.album_aware.write().await;

    *locked_album_aware = !*locked_album_aware;
    
    // This will invalidate cache if mode actually changed
    locked_song_queue.set_album_aware(*locked_album_aware);
    
    println!("[+] album-aware mode toggled to: {}", *locked_album_aware);

    // Use caching for instant response
    locked_song_queue.shuffle_and_add_with_cache(&*locked_tags_data, &mut *locked_mpd_conn);

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
