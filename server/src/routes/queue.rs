use rocket::serde::json::Json;
use rocket::{get, State, routes};
use crate::app_state::AppState;
use crate::mpd_conn::traits::{MpdClient, Song};
use crate::mpd_conn::mpd_pool::PooledMpdConnection;
use crate::models::song_queue::SongQueue;
use tokio::sync::MutexGuard;

pub fn routes() -> Vec<rocket::Route> {
    routes![get_queue, clear_queue]
}

#[get("/queue/all")]
pub async fn get_queue(app_state: &State<AppState>) -> Json<Vec<Song>> {
    let mut pooled_conn: PooledMpdConnection = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(vec![]),
    };
    
    let queue = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(q) => q,
        Err(_) => vec![],
    };
    
    Json(queue)
}

#[get("/queue/clear")]
pub async fn clear_queue(app_state: &State<AppState>) -> Json<bool> {
    let mut internal_queue: MutexGuard<SongQueue> = app_state.queue.lock().await;
    internal_queue.clear();
    Json(true)
}
