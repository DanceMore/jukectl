use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::models::hashable_song::HashableSong;
use crate::mpd_conn::mpd_conn::MpdConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

impl TagsData {
    // Get regular (non-album-aware) songs
    pub fn get_allowed_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        self.get_regular_songs(mpd_client)
    }

    // Get album-aware songs - this is a separate method to maintain separation of concerns
    pub fn get_album_aware_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        let mut album_songs = HashSet::new();

        // Get album representative songs from any tags
        for tag in &self.any {
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                println!("[+] fetching album representatives from tag {}", tag);

                for representative_song in playlist {
                    // For each representative song, get the full album
                    if let Some(album_name) = Self::get_tag_value(&representative_song, "Album") {
                        println!("[+] expanding album: {}", album_name);

                        // Get all songs from this album
                        let album_songs_result = self.get_songs_from_album(mpd_client, &album_name);
                        for song in album_songs_result {
                            album_songs.insert(HashableSong(song));
                        }
                    }
                }
            }
        }

        // Apply "not" filters
        let (_, not_tags) = self.tags_to_strings();
        for tag in &not_tags {
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                println!("[-] removing songs from tag {}", tag);
                for song in playlist {
                    album_songs.remove(&HashableSong(song));
                }
            }
        }

        album_songs
    }

    fn get_regular_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
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

    // Helper function to get a tag value from a song
    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags.iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    // Get all songs from a specific album
    fn get_songs_from_album(&self, mpd_client: &mut MpdConn, album_name: &str) -> Vec<mpd::Song> {
        // Use Term::Tag("album") to search by album tag
        let mut query = mpd::Query::new();
        query.and(mpd::Term::Tag("album".into()), album_name);

        match mpd_client.mpd.search(&query, None) {
            Ok(songs) => songs,
            Err(e) => {
                eprintln!("[!] Error searching for album '{}': {}", album_name, e);
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
