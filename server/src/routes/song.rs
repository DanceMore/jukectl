use rocket::serde::json::Json;
use crate::app_state::AppState;

#[derive(serde::Deserialize)]
struct SongTagsUpdate {
    add: Vec<String>,
    remove: Vec<String>,
}

#[post("/song/tags", data = "<song_tags>")]
async fn update_song_tags(
    song_tags: Json<SongTagsUpdate>,
    app_state: &rocket::State<AppState>,
) -> Json<String> {
    let add_tags = &song_tags.add; // Tags to add
    let remove_tags = &song_tags.remove; // Tags to remove

    // Get connection from pool
    let mut pooled_conn = match app_state.mpd_pool.get_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("[!] Error getting MPD connection from pool: {}", e);
            return Json("Error: Could not get MPD connection".to_string());
        }
    };

    // Get the first song from the now playing queue
    let first_song = match pooled_conn.mpd_conn().mpd.queue() {
        Ok(queue) => queue.first().cloned(),
        Err(e) => {
            eprintln!("[!] Failed to get MPD queue: {}", e);
            return Json("Error: Failed to get current song".to_string());
        }
    };

    if let Some(song) = first_song {
        for tag in add_tags {
            println!("[+] Add song to tag {}", tag);
            // Add the song to the playlist with the specified tag
            if let Err(error) = pooled_conn.mpd_conn().mpd.pl_push(tag, song.clone()) {
                eprintln!("[!] Error adding song to tag playlist: {}", error);
            }
        }

        // Start the timer
        let remove_tags_timer = std::time::Instant::now();
        for tag in remove_tags {
            println!("[!] Remove song from tag {}", tag);

            let fetch_timer = std::time::Instant::now();
            // Find the song's position(s) in the playlist with the specified tag
            let playlist = match pooled_conn.mpd_conn().mpd.playlist(tag) {
                Ok(playlist) => playlist,
                Err(err) => {
                    eprintln!("[!] Error getting playlist: {}", err);
                    continue;
                }
            };
            let fetch_timer_elapsed = fetch_timer.elapsed();
            println!("[-] network fetch took: {:?}", fetch_timer_elapsed);

            let delete_timer = std::time::Instant::now();
            let positions_to_delete: Vec<_> = playlist
                .iter()
                .enumerate()
                .filter(|(_, song_to_remove)| song_to_remove.file == song.file)
                .map(|(position, _)| position as u32)
                .collect();
            let delete_timer_elapsed = delete_timer.elapsed();
            println!("[-] innermost walk took: {:?}", delete_timer_elapsed);

            // Delete the songs at the found positions
            for position in positions_to_delete.iter() {
                if let Err(error) = pooled_conn.mpd_conn().mpd.pl_delete(tag, *position) {
                    eprintln!("Error removing song from tag playlist: {}", error);
                }
            }

            // Print how many times the song was removed
            if positions_to_delete.is_empty() {
                println!("Song not found in the playlist with tag {}", tag);
            } else {
                println!(
                    "Song removed {} times from the playlist with tag {}",
                    positions_to_delete.len(),
                    tag
                );
            }
        }

        // Stop the timer
        let elapsed_time = remove_tags_timer.elapsed();
        println!("[-] remove_tags took: {:?}", elapsed_time);
    }

    Json("Tags updated successfully".to_string())
}

// Return routes defined in this module
pub fn routes() -> Vec<rocket::Route> {
    routes![update_song_tags]
}
