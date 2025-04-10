use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::models::hashable_song::HashableSong;
use crate::mpd_conn::MpdConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

impl TagsData {
    pub fn get_allowed_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        let (any_tags, not_tags) = self.tags_to_strings();

        // Create a HashSet to store the desired songs
        let mut desired_songs = HashSet::new();

        // Process "any" tags
        for tag in &any_tags {
            let start_time1 = std::time::Instant::now();
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                let elapsed_time1 = start_time1.elapsed();
                println!("[-] fetching playlist took: {:?}", elapsed_time1);
                println!("[+] searching tag {} for songs to add", tag);
                for song in playlist {
                    desired_songs.insert(HashableSong(song));
                }
            }
        }

        // Process "not" tags
        for tag in &not_tags {
            let start_time2 = std::time::Instant::now();
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                let elapsed_time2 = start_time2.elapsed();
                println!("[-] fetching playlist took: {:?}", elapsed_time2);
                println!("[-] searching tag {} for songs to remove", tag);
                for song in playlist {
                    desired_songs.remove(&HashableSong(song));
                }
            }
        }

        desired_songs
    }

    fn tags_to_strings(&self) -> (HashSet<String>, HashSet<String>) {
        let any_tags: HashSet<String> = self
            .any
            .iter()
            .flat_map(|s| s.split(',').map(String::from))
            .collect();
        let not_tags: HashSet<String> = self
            .not
            .iter()
            .flat_map(|s| s.split(',').map(String::from))
            .collect();

        (any_tags, not_tags)
    }
}
