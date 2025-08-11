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

// Define your enhanced queue type with dual caching
pub struct SongQueue {
    inner: VecDeque<mpd::Song>,
    album_aware: bool,

    // Dual cache system
    regular_cache: Option<CacheEntry>,
    album_cache: Option<CacheEntry>,

    // Background precompute flags
    regular_precompute_pending: bool,
    album_precompute_pending: bool,

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
            regular_cache: None,
            album_cache: None,
            regular_precompute_pending: false,
            album_precompute_pending: false,
            cache_ttl: Duration::from_secs(600), // 10 minute cache
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn set_album_aware(&mut self, album_aware: bool) {
        if self.album_aware != album_aware {
            println!(
                "[+] Album-aware mode changed to {}, invalidating caches",
                album_aware
            );
        }
        self.album_aware = album_aware;
    }

    // Mark that we need background precomputation
    pub fn request_precompute(&mut self, album_mode: bool) {
        if album_mode {
            self.album_precompute_pending = true;
            println!("[+] Album cache precompute requested");
        } else {
            self.regular_precompute_pending = true;
            println!("[+] Regular cache precompute requested");
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.regular_cache = None;
        self.album_cache = None;
        println!("[+] All caches invalidated");
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

    // Enhanced hash function that includes album_aware mode
    fn hash_tags_with_mode(
        tags_data: &crate::models::tags_data::TagsData,
        album_aware: bool,
    ) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        tags_data.any.hash(&mut hasher);
        tags_data.not.hash(&mut hasher);
        album_aware.hash(&mut hasher); // Include album mode in hash!
        hasher.finish()
    }

    // Updated cache validation
    fn is_cache_valid(&self, tags_data: &crate::models::tags_data::TagsData) -> bool {
        let current_hash = Self::hash_tags_with_mode(tags_data, self.album_aware);

        if self.album_aware {
            self.album_cache
                .as_ref()
                .map_or(false, |c| c.is_valid(current_hash, self.cache_ttl))
        } else {
            self.regular_cache
                .as_ref()
                .map_or(false, |c| c.is_valid(current_hash, self.cache_ttl))
        }
    }

    // Fixed shuffle_and_add_with_cache_async
    pub async fn shuffle_and_add_with_cache_async(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let start_time = Instant::now();
        let tags_hash = Self::hash_tags_with_mode(tags_data, self.album_aware);

        println!(
            "[+] Cache lookup - album_aware: {}, hash: {}",
            self.album_aware, tags_hash
        );

        let songs = if self.album_aware {
            if let Some(ref cache) = self.album_cache {
                if cache.is_valid(tags_hash, self.cache_ttl) {
                    self.cache_hits += 1;
                    println!(
                        "[+] CACHE HIT: Using cached album-aware songs ({} songs)",
                        cache.songs.len()
                    );
                    cache.songs.clone()
                } else {
                    self.cache_misses += 1;
                    println!("[+] CACHE MISS: Album cache invalid/stale, rebuilding...");
                    let fresh_songs = tags_data.get_album_aware_songs_parallel(mpd_client).await;
                    self.album_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                    fresh_songs
                }
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Building album-aware cache...");
                let fresh_songs = tags_data.get_album_aware_songs_parallel(mpd_client).await;
                self.album_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        } else {
            if let Some(ref cache) = self.regular_cache {
                if cache.is_valid(tags_hash, self.cache_ttl) {
                    self.cache_hits += 1;
                    println!(
                        "[+] CACHE HIT: Using cached regular songs ({} songs)",
                        cache.songs.len()
                    );
                    cache.songs.clone()
                } else {
                    self.cache_misses += 1;
                    println!("[+] CACHE MISS: Regular cache invalid/stale, rebuilding...");
                    let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                    self.regular_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                    fresh_songs
                }
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Building regular cache...");
                let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                self.regular_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        };

        println!("[+] Using {} songs for shuffle", songs.len());

        // Use existing shuffle logic
        if self.album_aware {
            self.shuffle_and_add_album_aware(songs);
        } else {
            self.shuffle_and_add_regular(songs);
        }

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

        let songs = if self.album_aware {
            if let Some(ref cache) = self.album_cache {
                if cache.is_valid(tags_hash, self.cache_ttl) {
                    self.cache_hits += 1;
                    println!(
                        "[+] CACHE HIT: Using cached album-aware songs ({} songs)",
                        cache.songs.len()
                    );
                    cache.songs.clone()
                } else {
                    self.cache_misses += 1;
                    println!("[+] CACHE MISS: Album cache invalid, using sync refresh...");
                    let fresh_songs = tags_data.get_album_aware_songs(mpd_client);
                    self.album_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                    fresh_songs
                }
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Building album-aware cache...");
                let fresh_songs = tags_data.get_album_aware_songs(mpd_client);
                self.album_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        } else {
            if let Some(ref cache) = self.regular_cache {
                if cache.is_valid(tags_hash, self.cache_ttl) {
                    self.cache_hits += 1;
                    println!(
                        "[+] CACHE HIT: Using cached regular songs ({} songs)",
                        cache.songs.len()
                    );
                    cache.songs.clone()
                } else {
                    self.cache_misses += 1;
                    println!("[+] CACHE MISS: Regular cache invalid, refreshing...");
                    let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                    self.regular_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                    fresh_songs
                }
            } else {
                self.cache_misses += 1;
                println!("[+] CACHE MISS: Building regular cache...");
                let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                self.regular_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
                fresh_songs
            }
        };

        if self.album_aware {
            self.shuffle_and_add_album_aware(songs);
        } else {
            self.shuffle_and_add_regular(songs);
        }

        println!("[+] Sync shuffle_and_add took: {:?}", start_time.elapsed());
    }

    // Background precompute method for scheduler
    pub async fn background_precompute(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let tags_hash = Self::hash_tags(tags_data);

        // Check if regular precompute is needed
        if self.regular_precompute_pending
            || self
                .regular_cache
                .as_ref()
                .map_or(true, |c| !c.is_valid(tags_hash, self.cache_ttl))
        {
            println!("[+] Background: Precomputing regular cache...");
            let start = Instant::now();
            let fresh_songs = tags_data.get_allowed_songs(mpd_client);
            self.regular_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
            self.regular_precompute_pending = false;
            println!(
                "[+] Background: Regular cache precomputed in {:?} ({} songs)",
                start.elapsed(),
                fresh_songs.len()
            );
        }

        // Check if album precompute is needed
        if self.album_precompute_pending
            || self
                .album_cache
                .as_ref()
                .map_or(true, |c| !c.is_valid(tags_hash, self.cache_ttl))
        {
            println!("[+] Background: Precomputing album cache with parallel expansion...");
            let start = Instant::now();
            let fresh_songs = tags_data.get_album_aware_songs_parallel(mpd_client).await;
            self.album_cache = Some(CacheEntry::new(fresh_songs.clone(), tags_hash));
            self.album_precompute_pending = false;
            println!(
                "[+] Background: Album cache precomputed in {:?} ({} songs)",
                start.elapsed(),
                fresh_songs.len()
            );
        }
    }

    // Rest of your existing methods remain the same
    pub fn add(&mut self, song: mpd::Song) {
        self.inner.push_back(song);
    }

    pub fn remove(&mut self) -> Option<mpd::Song> {
        self.inner.pop_front()
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

    pub fn shuffle_and_add(&mut self, songs: HashSet<HashableSong>) {
        if self.album_aware {
            self.shuffle_and_add_album_aware(songs);
        } else {
            self.shuffle_and_add_regular(songs);
        }
    }

    fn shuffle_and_add_regular(&mut self, songs: HashSet<HashableSong>) {
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
        println!("[-] shuffle_and_add_regular took: {:?}", elapsed_time);
    }

    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags
            .iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    fn shuffle_and_add_album_aware(&mut self, songs: HashSet<HashableSong>) {
        let start_time = std::time::Instant::now();
        self.empty_queue();

        let mut albums: HashMap<String, Vec<mpd::Song>> = HashMap::new();

        for hashable_song in songs {
            let song = mpd::Song::from(hashable_song);
            let album_name =
                Self::get_tag_value(&song, "Album").unwrap_or_else(|| "Unknown Album".to_string());

            albums.entry(album_name).or_insert_with(Vec::new).push(song);
        }

        for (album_name, album_songs) in albums.iter_mut() {
            album_songs.sort_by(|a, b| {
                let track_a = Self::get_tag_value(a, "Track")
                    .and_then(|t| t.parse::<u32>().ok())
                    .unwrap_or(0);
                let track_b = Self::get_tag_value(b, "Track")
                    .and_then(|t| t.parse::<u32>().ok())
                    .unwrap_or(0);
                track_a.cmp(&track_b)
            });
        }

        let mut album_names: Vec<String> = albums.keys().cloned().collect();
        let mut rng = rand::rng();
        album_names.shuffle(&mut rng);

        for album_name in album_names {
            if let Some(album_songs) = albums.remove(&album_name) {
                for song in album_songs {
                    self.add(song);
                }
            }
        }

        let elapsed_time = start_time.elapsed();
        println!("[-] album-aware shuffle_and_add took: {:?}", elapsed_time);
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

    pub fn has_valid_cache(&self, tags_data: &crate::models::tags_data::TagsData) -> (bool, bool) {
        let tags_hash = Self::hash_tags(tags_data);
        let regular_valid = self
            .regular_cache
            .as_ref()
            .map_or(false, |c| c.is_valid(tags_hash, self.cache_ttl));
        let album_valid = self
            .album_cache
            .as_ref()
            .map_or(false, |c| c.is_valid(tags_hash, self.cache_ttl));
        (regular_valid, album_valid)
    }
}
