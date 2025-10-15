use rand::seq::SliceRandom;
use std::collections::VecDeque;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::models::hashable_song::HashableSong;

// Enhanced cache entry with metadata
#[derive(Clone)]
struct CacheEntry {
    songs: HashSet<HashableSong>,
    timestamp: Instant,
    tags_hash: u64, // Simple hash of tags to detect changes
}

impl CacheEntry {
    fn new(songs: HashSet<HashableSong>, tags_hash: u64) -> Self {
        Self {
            songs,
            timestamp: Instant::now(),
            tags_hash,
        }
    }

    fn is_valid(&self, current_tags_hash: u64, ttl: Duration) -> bool {
        self.tags_hash == current_tags_hash && self.timestamp.elapsed() < ttl
    }
}

// Define your enhanced queue type with caching
pub struct SongQueue {
    inner: VecDeque<mpd::Song>,
    album_aware: bool,

    // Single cache system (shuffle is the same for both modes!)
    cache: Option<CacheEntry>,

    // Background precompute flag
    precompute_pending: bool,

    cache_ttl: Duration,

    // Performance metrics
    cache_hits: u64,
    cache_misses: u64,
}

impl SongQueue {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            album_aware: false,
            cache: None,
            precompute_pending: false,
            cache_ttl: Duration::from_secs(600), // 10 minute cache
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn set_album_aware(&mut self, album_aware: bool) {
        if self.album_aware != album_aware {
            println!(
                "[+] Album-aware mode changed to {}",
                album_aware
            );
        }
        self.album_aware = album_aware;
    }

    // Mark that we need background precomputation
    pub fn request_precompute(&mut self) {
        self.precompute_pending = true;
        println!("[+] Cache precompute requested");
    }

    pub fn invalidate_cache(&mut self) {
        self.cache = None;
        println!("[+] Cache invalidated");
    }

    // Simple hash function for tags
    fn hash_tags(tags_data: &crate::models::tags_data::TagsData) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tags_data.any.hash(&mut hasher);
        tags_data.not.hash(&mut hasher);
        hasher.finish()
    }

    // Main shuffle method with caching
    pub async fn shuffle_and_add_with_cache_async(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let start_time = Instant::now();
        let tags_hash = Self::hash_tags(tags_data);

        println!("[+] Cache lookup - hash: {}", tags_hash);

        // Check cache validity
        let songs = if let Some(ref cache) = self.cache {
            if cache.is_valid(tags_hash, self.cache_ttl) {
                self.cache_hits += 1;
                println!(
                    "[+] CACHE HIT: Using cached songs ({} songs)",
                    cache.songs.len()
                );
                cache.songs.clone()
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Cache invalid/stale, rebuilding...");
                let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                self.cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        } else {
            self.cache_misses += 1;
            println!("[+] CACHE MISS: Building cache...");
            let fresh_songs = tags_data.get_allowed_songs(mpd_client);
            self.cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
            fresh_songs
        };

        println!("[+] Using {} songs for shuffle", songs.len());

        // Shuffle is the same for both modes!
        self.shuffle_and_add(songs);

        println!("[+] Total shuffle_and_add took: {:?}", start_time.elapsed());
    }

    // Synchronous fallback method
    pub fn shuffle_and_add_with_cache(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let start_time = Instant::now();
        let tags_hash = Self::hash_tags(tags_data);

        let songs = if let Some(ref cache) = self.cache {
            if cache.is_valid(tags_hash, self.cache_ttl) {
                self.cache_hits += 1;
                println!(
                    "[+] CACHE HIT: Using cached songs ({} songs)",
                    cache.songs.len()
                );
                cache.songs.clone()
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Cache invalid, refreshing...");
                let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                self.cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        } else {
            self.cache_misses += 1;
            println!("[+] CACHE MISS: Building cache...");
            let fresh_songs = tags_data.get_allowed_songs(mpd_client);
            self.cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
            fresh_songs
        };

        self.shuffle_and_add(songs);

        println!("[+] Sync shuffle_and_add took: {:?}", start_time.elapsed());
    }

    // Background precompute method for scheduler
    pub async fn background_precompute(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let tags_hash = Self::hash_tags(tags_data);

        // Check if precompute is needed
        if self.precompute_pending
            || self
                .cache
                .as_ref()
                .map_or(true, |c| !c.is_valid(tags_hash, self.cache_ttl))
        {
            println!("[+] Background: Precomputing cache...");
            let start = Instant::now();
            let fresh_songs = tags_data.get_allowed_songs(mpd_client);
            self.cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
            self.precompute_pending = false;
            println!(
                "[+] Background: Cache precomputed in {:?} ({} songs)",
                start.elapsed(),
                fresh_songs.len()
            );
        }
    }

    pub fn add(&mut self, song: mpd::Song) {
        self.inner.push_back(song);
    }

    // Regular remove - pops one song
    pub fn remove(&mut self) -> Option<mpd::Song> {
        self.inner.pop_front()
    }

    // Album-aware remove - pops one song, returns full album
    pub fn remove_album_aware(
        &mut self,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) -> Option<Vec<mpd::Song>> {
        // Pop the seed song
        let seed_song = self.inner.pop_front()?;

        // Get the album name from the seed song
        let album_name = Self::get_tag_value(&seed_song, "Album")
            .unwrap_or_else(|| "Unknown Album".to_string());

        println!("[+] Album-aware dequeue: Loading full album '{}'", album_name);

        // Query MPD for all songs from this album using proper Query API
        let album_songs = {
            let mut query = mpd::Query::new();
            query.and(mpd::Term::Tag("album".into()), album_name.as_str());
            
            match mpd_client.mpd.search(&query, None) {
                Ok(songs) => songs,
                Err(e) => {
                    eprintln!("[-] Error querying album songs: {}", e);
                    // Fallback: return just the seed song
                    return Some(vec![seed_song]);
                }
            }
        };

        // Sort by track number
        let mut sorted_songs = album_songs;
        sorted_songs.sort_by(|a, b| {
            let track_a = Self::get_tag_value(a, "Track")
                .and_then(|t| t.parse::<u32>().ok())
                .unwrap_or(0);
            let track_b = Self::get_tag_value(b, "Track")
                .and_then(|t| t.parse::<u32>().ok())
                .unwrap_or(0);
            track_a.cmp(&track_b)
        });

        println!("[+] Loaded {} tracks from album", sorted_songs.len());

        Some(sorted_songs)
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn head(&self, count: Option<usize>) -> Vec<mpd::Song> {
        let count = count.unwrap_or(3);
        self.inner.iter().take(count).cloned().collect()
    }

    pub fn tail(&self, count: Option<usize>) -> Vec<mpd::Song> {
        let len = self.inner.len();
        let count = count.unwrap_or(3);
        self.inner
            .iter()
            .skip(len.saturating_sub(count))
            .cloned()
            .collect()
    }

    pub fn empty_queue(&mut self) {
        self.inner.clear();
    }

    // Simplified shuffle - same for both modes!
    pub fn shuffle_and_add(&mut self, songs: HashSet<HashableSong>) {
        let start_time = std::time::Instant::now();
        self.inner.reserve(songs.len());
        self.empty_queue();

        let mut song_vec: Vec<mpd::Song> = songs.into_iter().map(mpd::Song::from).collect();
        let mut rng = rand::rng();
        song_vec.shuffle(&mut rng);

        for song in song_vec {
            self.add(song);
        }

        let elapsed_time = start_time.elapsed();
        println!("[+] Shuffle and add took: {:?}", elapsed_time);
    }

    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags
            .iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    // Performance monitoring methods
    pub fn cache_stats(&self) -> (u64, u64, f64) {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        (self.cache_hits, self.cache_misses, hit_rate)
    }

    // Simplified - single cache, single boolean
    pub fn has_valid_cache(&self, tags_data: &crate::models::tags_data::TagsData) -> bool {
        let tags_hash = Self::hash_tags(tags_data);
        self.cache
            .as_ref()
            .map_or(false, |c| c.is_valid(tags_hash, self.cache_ttl))
    }
}
