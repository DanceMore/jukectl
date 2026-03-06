use rocket::serde::json::Json;
use rocket::{get, State, routes};
use crate::app_state::AppState;
use crate::mpd_conn::traits::{MpdClient, Song};
use crate::models::tags_data::TagsResponse;
use crate::mpd_conn::mpd_pool::PooledMpdConnection;

pub fn routes() -> Vec<rocket::Route> {
    routes![get_tags, get_tag_songs]
}

#[get("/tags")]
pub async fn get_tags(app_state: &State<AppState>) -> Json<TagsResponse> {
    let mut pooled_conn: PooledMpdConnection = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(TagsResponse::new()),
    };
    
    let songs = pooled_conn.mpd_conn().mpd.listall().unwrap_or_default();
    let playlists = pooled_conn.mpd_conn().mpd.playlists().unwrap_or_default();
    
    Json(TagsResponse::to_api_response(songs, playlists))
}

#[get("/tags/<tag>")]
pub async fn get_tag_songs(app_state: &State<AppState>, tag: String) -> Json<Vec<Song>> {
    let mut pooled_conn: PooledMpdConnection = match app_state.mpd_pool.get_connection().await {
        Ok(c) => c,
        Err(_) => return Json(vec![]),
    };
    
    if let Ok(songs) = pooled_conn.mpd_conn().mpd.playlist(&tag) {
        if !songs.is_empty() {
            return Json(songs);
        }
    }
    
    let songs = pooled_conn.mpd_conn().mpd.listall().unwrap_or_default();
    let filtered: Vec<Song> = songs.into_iter().filter(|s| {
        s.artist.as_deref() == Some(&tag) || s.album.as_deref() == Some(&tag)
    }).collect();
    
    Json(filtered)
}
