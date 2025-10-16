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
}

#[post("/tags", data = "<tags_update>")]
async fn update_tags(
    tags_update: Json<TagsUpdate>,
    app_state: &rocket::State<AppState>,
) -> Json<TagsResponse> {
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            // Return current tags even if we can't update the queue
            let locked_tags_data = app_state.tags_data.read().await;
            let album_aware = *app_state.album_aware.read().await;
            return Json(TagsResponse {
                any: locked_tags_data.any.clone(),
                not: locked_tags_data.not.clone(),
                album_aware,
            });
        }
    };

    let mut locked_song_queue = app_state.song_queue.write().await;
    let mut locked_tags_data = app_state.tags_data.write().await;

    // Update tags if provided
    if let Some(any) = &tags_update.any {
        locked_tags_data.any = any.clone();
        locked_song_queue.invalidate_cache();
    }
    if let Some(not) = &tags_update.not {
        locked_tags_data.not = not.clone();
        locked_song_queue.invalidate_cache();
    }

    // Rebuild queue with new tags
    locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());

    let album_aware = *app_state.album_aware.read().await;

    Json(TagsResponse {
        any: locked_tags_data.any.clone(),
        not: locked_tags_data.not.clone(),
        album_aware,
    })
}

#[post("/album-mode/<enabled>")]
async fn set_album_mode(
    enabled: bool,
    app_state: &rocket::State<AppState>,
) -> Json<serde_json::Value> {
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json(serde_json::json!({
                "error": "Could not get MPD connection",
                "album_aware": enabled
            }));
        }
    };

    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;
    let mut locked_album_aware = app_state.album_aware.write().await;

    *locked_album_aware = enabled;
    locked_song_queue.set_album_aware(enabled);

    println!("[+] Album-aware mode set to: {}", enabled);

    // Rebuild queue (dequeue behavior changes, but queue content stays same)
    locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());

    Json(serde_json::json!({
        "album_aware": enabled,
        "message": if enabled { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    }))
}

#[post("/album-mode/toggle")]
async fn toggle_album_mode(app_state: &rocket::State<AppState>) -> Json<serde_json::Value> {
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            let current_mode = *app_state.album_aware.read().await;
            return Json(serde_json::json!({
                "error": "Could not get MPD connection",
                "album_aware": current_mode
            }));
        }
    };

    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;
    let mut locked_album_aware = app_state.album_aware.write().await;

    *locked_album_aware = !*locked_album_aware;
    locked_song_queue.set_album_aware(*locked_album_aware);

    println!("[+] Album-aware mode toggled to: {}", *locked_album_aware);

    locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());

    Json(serde_json::json!({
        "album_aware": *locked_album_aware,
        "message": if *locked_album_aware { "Album-aware mode enabled" } else { "Album-aware mode disabled" }
    }))
}

#[get("/cache-stats")]
async fn cache_stats(app_state: &rocket::State<AppState>) -> Json<serde_json::Value> {
    let locked_song_queue = app_state.song_queue.read().await;
    let locked_tags_data = app_state.tags_data.read().await;

    let (hits, misses, hit_rate) = locked_song_queue.cache_stats();
    let cache_valid = locked_song_queue.has_valid_cache(&*locked_tags_data);
    let queue_length = locked_song_queue.len();
    let album_aware = *app_state.album_aware.read().await;

    Json(serde_json::json!({
        "cache_hits": hits,
        "cache_misses": misses,
        "hit_rate_percent": hit_rate,
        "cache_valid": cache_valid,
        "queue_length": queue_length,
        "album_aware_enabled": album_aware,
        "status": if hit_rate > 80.0 { "excellent" } else if hit_rate > 60.0 { "good" } else { "needs_optimization" }
    }))
}

#[post("/cache/refresh")]
async fn refresh_cache(app_state: &rocket::State<AppState>) -> Json<serde_json::Value> {
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json(serde_json::json!({
                "error": "Could not get MPD connection"
            }));
        }
    };

    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;

    println!("[+] Manual cache refresh requested");
    let start_time = std::time::Instant::now();

    // Invalidate and rebuild
    locked_song_queue.invalidate_cache();
    locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());

    let elapsed = start_time.elapsed();

    Json(serde_json::json!({
        "status": "success",
        "refresh_time_ms": elapsed.as_millis(),
        "queue_length": locked_song_queue.len(),
        "message": "Cache refreshed successfully"
    }))
}

// Return routes defined in this module
pub fn routes() -> Vec<rocket::Route> {
    routes![
        tags,
        update_tags,
        set_album_mode,
        toggle_album_mode,
        cache_stats,
        refresh_cache
    ]
}
