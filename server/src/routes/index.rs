use rocket::serde::json::Json;
use rocket::{get, State, routes};
use crate::app_state::AppState;
use crate::mpd_conn::traits::{MpdClient, Song};

pub fn routes() -> Vec<rocket::Route> {
    routes![index]
}

fn queue_to_filenames(song_array: Vec<Song>) -> Vec<String> {
    song_array.into_iter().map(|s| s.file).collect()
}

#[get("/")]
pub async fn index(app_state: &State<AppState>) -> Json<Vec<String>> {
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(vec![]),
    };
    
    let song_array: Vec<Song> = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(s) => s,
        Err(_) => vec![],
    };
    
    Json(queue_to_filenames(song_array))
}
