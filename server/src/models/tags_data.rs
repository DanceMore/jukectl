use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::task::JoinSet;

use crate::models::hashable_song::HashableSong;
use crate::mpd_conn::mpd_conn::MpdConn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

impl TagsData {
    const MAX_ALBUMS_ALBUM_AWARE: usize = 150;
    const PARALLEL_BATCH_SIZE: usize = 20; // Process albums in parallel batches

    // Get regular (non-album-aware) songs
    pub fn get_allowed_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        self.get_regular_songs(mpd_client)
    }

    // Fixed version of get_album_aware_songs_parallel in tags_data.rs
    pub async fn get_album_aware_songs_parallel(
        &self,
        mpd_client: &mut MpdConn,
    ) -> HashSet<HashableSong> {
        let start_time = Instant::now();

        // FIRST: Get the base set of allowed songs (respecting current tags)
        let allowed_songs = self.get_allowed_songs(mpd_client);
        println!(
            "[+] Base allowed songs from current tags: {}",
            allowed_songs.len()
        );

        // SECOND: Group allowed songs by album
        let mut albums: HashMap<String, Vec<HashableSong>> = HashMap::new();
        for song in allowed_songs {
            let album_name = Self::get_tag_value(&song.0, "Album")
                .unwrap_or_else(|| "Unknown Album".to_string());
            albums.entry(album_name).or_insert_with(Vec::new).push(song);
        }

        println!("[+] Found {} albums from allowed songs", albums.len());

        // THIRD: For each album, expand to get ALL songs in that album (but still filter by tags)
        let mut all_album_songs = HashSet::new();
        let album_names: Vec<String> = albums
            .keys()
            .take(Self::MAX_ALBUMS_ALBUM_AWARE)
            .cloned()
            .collect();

        // Process albums in parallel batches
        let chunks: Vec<_> = album_names.chunks(Self::PARALLEL_BATCH_SIZE).collect();

        for (batch_idx, chunk) in chunks.iter().enumerate() {
            println!(
                "[+] Processing batch {}/{} ({} albums)",
                batch_idx + 1,
                chunks.len(),
                chunk.len()
            );

            let batch_start = Instant::now();
            let mut join_set = JoinSet::new();

            // Clone the tags for filtering in parallel tasks
            let any_tags = self.any.clone();
            let not_tags = self.not.clone();

            for album_name in chunk.iter() {
                let album_name = album_name.clone();
                let any_tags = any_tags.clone();
                let not_tags = not_tags.clone();

                join_set.spawn(async move {
                    match MpdConn::new() {
                        Ok(mut client) => {
                            // Get all songs in this album
                            let album_songs =
                                Self::get_songs_from_album_static(&mut client, &album_name)
                                    .unwrap_or_else(|_| Vec::new());

                            // Filter the album songs by current tags
                            Self::filter_songs_by_tags(
                                album_songs,
                                &any_tags,
                                &not_tags,
                                &mut client,
                            )
                        }
                        Err(e) => {
                            eprintln!(
                                "[!] Failed to create MPD connection for album '{}': {}",
                                album_name, e
                            );
                            Vec::new()
                        }
                    }
                });
            }

            // Collect results from this batch
            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(songs) => {
                        for song in songs {
                            all_album_songs.insert(HashableSong(song));
                        }
                    }
                    Err(e) => eprintln!("[!] Task join error: {}", e),
                }
            }

            println!(
                "[+] Batch {} completed in {:?}",
                batch_idx + 1,
                batch_start.elapsed()
            );
        }

        println!(
            "[+] Parallel album expansion completed in {:?} - {} total songs",
            start_time.elapsed(),
            all_album_songs.len()
        );

        all_album_songs
    }

    // New helper method to filter songs by tags within parallel tasks
    fn filter_songs_by_tags(
        songs: Vec<mpd::Song>,
        any_tags: &[String],
        not_tags: &[String],
        mpd_client: &mut MpdConn,
    ) -> Vec<mpd::Song> {
        let mut allowed_files = HashSet::new();
        let mut forbidden_files = HashSet::new();

        // Get allowed files from "any" tags
        for tag in any_tags {
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                for song in playlist {
                    allowed_files.insert(song.file);
                }
            }
        }

        // Get forbidden files from "not" tags
        for tag in not_tags {
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                for song in playlist {
                    forbidden_files.insert(song.file);
                }
            }
        }

        // Filter the songs
        songs
            .into_iter()
            .filter(|song| {
                allowed_files.contains(&song.file) && !forbidden_files.contains(&song.file)
            })
            .collect()
    }

    // Fallback to original method for compatibility
    pub fn get_album_aware_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        let mut album_songs = HashSet::new();
        let mut processed_albums = HashSet::new();
        let mut album_count = 0;

        for tag in &self.any {
            if let Ok(playlist) = mpd_client.mpd.playlist(tag) {
                println!("[+] fetching album representatives from tag {}", tag);

                let mut representative_songs: Vec<_> = playlist.into_iter().collect();
                let mut rng = rand::rng();
                representative_songs.shuffle(&mut rng);

                for representative_song in representative_songs {
                    if album_count >= Self::MAX_ALBUMS_ALBUM_AWARE {
                        println!(
                            "[!] Hit album limit of {}, stopping album expansion",
                            Self::MAX_ALBUMS_ALBUM_AWARE
                        );
                        break;
                    }

                    if let Some(album_name) = Self::get_tag_value(&representative_song, "Album") {
                        if processed_albums.contains(&album_name) {
                            continue;
                        }

                        processed_albums.insert(album_name.clone());
                        album_count += 1;

                        println!(
                            "[+] expanding album {}/{}: {}",
                            album_count,
                            Self::MAX_ALBUMS_ALBUM_AWARE,
                            album_name
                        );

                        let album_songs_result = self.get_songs_from_album(mpd_client, &album_name);
                        for song in album_songs_result {
                            album_songs.insert(HashableSong(song));
                        }
                    }
                }

                if album_count >= Self::MAX_ALBUMS_ALBUM_AWARE {
                    break;
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

        println!(
            "[+] Album-aware: {} albums processed, {} total songs",
            processed_albums.len(),
            album_songs.len()
        );
        album_songs
    }

    fn get_regular_songs(&self, mpd_client: &mut MpdConn) -> HashSet<HashableSong> {
        let (any_tags, not_tags) = self.tags_to_strings();
        let mut desired_songs = HashSet::new();

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

    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags
            .iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    fn get_songs_from_album(&self, mpd_client: &mut MpdConn, album_name: &str) -> Vec<mpd::Song> {
        Self::get_songs_from_album_static(mpd_client, album_name).unwrap_or_else(|e| {
            eprintln!("[!] Error searching for album '{}': {}", album_name, e);
            Vec::new()
        })
    }

    // Static version for use in async tasks
    fn get_songs_from_album_static(
        mpd_client: &mut MpdConn,
        album_name: &str,
    ) -> Result<Vec<mpd::Song>, Box<dyn std::error::Error + Send + Sync>> {
        let mut query = mpd::Query::new();
        query.and(mpd::Term::Tag("album".into()), album_name);

        Ok(mpd_client.mpd.search(&query, None)?)
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
