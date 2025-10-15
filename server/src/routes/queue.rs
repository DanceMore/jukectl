use crate::app_state::AppState;
use rocket::serde::json::Json;

// TODO: refactor this out
fn queue_to_filenames(song_array: Vec<mpd::Song>) -> Vec<String> {
    song_array.into_iter().map(|song| song.file).collect()
}

#[derive(serde::Serialize)]
struct QueueResponse {
    length: usize,
    head: Vec<String>,
    tail: Vec<String>,
}

// Option<> on count makes the Query Param optional :)
#[get("/queue?<count>")]
async fn get_queue(
    app_state: &rocket::State<AppState>,
    count: Option<usize>,
) -> Json<QueueResponse> {
    let count_value = count.unwrap_or(3); // Use a default value of 3 if count is None

    let locked_song_queue = app_state.song_queue.read().await;
    let length = locked_song_queue.len(); // Get the length of the queue

    // TODO: I kinda hate this presentation layer formatting, but it compiles...
    let head = locked_song_queue
        .head(Some(count_value))
        .iter()
        .map(|song| song.file.clone())
        .collect::<Vec<_>>();
    let tail = locked_song_queue
        .tail(Some(count_value))
        .iter()
        .map(|song| song.file.clone())
        .collect::<Vec<_>>();

    let res = QueueResponse { length, head, tail };
    Json(res)
}

#[derive(rocket::serde::Serialize)]
struct ShuffleResponse {
    old: Vec<String>,
    new: Vec<String>,
}

#[post("/shuffle")]
async fn shuffle_songs(app_state: &rocket::State<AppState>) -> Json<ShuffleResponse> {
    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json(ShuffleResponse {
                old: Vec::new(),
                new: Vec::new(),
            });
        }
    };

    let mut locked_song_queue = app_state.song_queue.write().await;
    let locked_tags_data = app_state.tags_data.read().await;

    // Capture the old songs before shuffling
    let old_songs = locked_song_queue.head(None);

    // Use the new async caching method with pooled connection
    locked_song_queue
        .shuffle_and_add_with_cache_async(&*locked_tags_data, pooled_conn.mpd_conn())
        .await;

    // Capture the new songs after shuffling
    let new_songs = locked_song_queue.head(None);

    let response = ShuffleResponse {
        old: queue_to_filenames(old_songs),
        new: queue_to_filenames(new_songs),
    };

    Json(response)
}

// Return routes defined in this module
pub fn routes() -> Vec<rocket::Route> {
    routes![get_queue, shuffle_songs]
}
