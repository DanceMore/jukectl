use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::models::hashable_song::HashableSong;
use crate::mpd_conn::mpd_conn::MpdConn;
use crate::mpd_conn::traits::MpdClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

impl TagsData {
    // Get songs based on tag filters
    // This is used for BOTH regular and album-aware modes
    // Album expansion happens at dequeue time, not here
    pub fn get_allowed_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        let (any_tags, not_tags) = self.tags_to_strings();
        let mut desired_songs = HashSet::new();

        // Add songs from "any" tags
        for tag in &any_tags {
            if Self::is_adhoc_playlist(tag) {
                println!("[!] skipping ad-hoc playlist in jukectl tag logic: {}", tag);
                continue;
            }

            let start_time = std::time::Instant::now();
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                let elapsed_time = start_time.elapsed();
                println!("[-] fetching playlist took: {:?}", elapsed_time);
                println!("[+] searching tag {} for songs to add", tag);
                for song in playlist {
                    desired_songs.insert(HashableSong(song));
                }
            }
        }

        // Remove songs from "not" tags
        for tag in &not_tags {
            if Self::is_adhoc_playlist(tag) {
                continue;
            }

            let start_time = std::time::Instant::now();
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                let elapsed_time = start_time.elapsed();
                println!("[-] fetching playlist took: {:?}", elapsed_time);
                println!("[-] searching tag {} for songs to remove", tag);
                for song in playlist {
                    desired_songs.remove(&HashableSong(song));
                }
            }
        }

        desired_songs
    }

    /// Check if a playlist name looks like an ad-hoc date-based playlist
    /// Patterns to exclude:
    /// yyyy-mm (e.g., 2024-03)
    /// yyyy-mm-dd (e.g., 2024-03-05)
    pub fn is_adhoc_playlist(name: &str) -> bool {
        let parts: Vec<&str> = name.split('-').collect();
        if parts.len() < 2 || parts.len() > 3 {
            return false;
        }

        // Check if the first part is a 4-digit year
        if parts[0].len() != 4 || !parts[0].chars().all(|c| c.is_ascii_digit()) {
            return false;
        }

        // Check if subsequent parts are 2-digit month/day
        for part in &parts[1..] {
            if part.len() != 2 || !part.chars().all(|c| c.is_ascii_digit()) {
                return false;
            }
        }

        true
    }

    /// List all available playlists from MPD that aren't ad-hoc playlists
    pub fn list_available_tags(mpd_client: &mut MpdConn) -> Vec<String> {
        match mpd_client.mpd.playlists() {
            Ok(playlists) => {
                let p: Vec<mpd::Playlist> = playlists;
                p.into_iter()
                    .map(|p| p.name)
                    .filter(|name| !Self::is_adhoc_playlist(name))
                    .collect()
            }
            Err(e) => {
                eprintln!("[!] Error listing playlists: {}", e);
                Vec::new()
            }
        }
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
