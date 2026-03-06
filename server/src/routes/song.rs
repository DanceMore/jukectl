use rocket::serde::json::Json;
use rocket::{get, State, routes};
use crate::app_state::AppState;
use crate::mpd_conn::traits::{MpdClient, Song};
use crate::mpd_conn::mpd_pool::PooledMpdConnection;

pub fn routes() -> Vec<rocket::Route> {
    routes![now_playing, list_all]
}

#[get("/song/now")]
pub async fn now_playing(app_state: &State<AppState>) -> Json<Option<Song>> {
    let mut pooled_conn: PooledMpdConnection = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(None),
    };
    
    let queue: Vec<Song> = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(q) => q,
        Err(_) => return Json(None),
    };
    
    Json(queue.first().cloned())
}

#[get("/song/all")]
pub async fn list_all(app_state: &State<AppState>) -> Json<Vec<Song>> {
    let mut pooled_conn: PooledMpdConnection = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(vec![]),
    };
    
    let songs = match pooled_conn.mpd_conn().mpd.listall() {
        Ok(s) => s,
        Err(_) => vec![],
    };
    
    Json(songs)
}
