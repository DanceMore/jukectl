use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::models::hashable_song::HashableSong;

// Define your custom queue type with caching
pub struct SongQueue {
    inner: VecDeque<mpd::Song>,
    album_aware: bool,
    // Caching system for both modes
    regular_cache: Option<HashSet<HashableSong>>,
    album_cache: Option<HashSet<HashableSong>>,
    cache_timestamp: Option<Instant>,
    cache_ttl: Duration,
}

impl SongQueue {
    // Initialize a new SongQueue with caching
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            album_aware: false,
            regular_cache: None,
            album_cache: None,
            cache_timestamp: None,
            cache_ttl: Duration::from_secs(300), // 5 minute cache
        }
    }

    // Set album-aware mode and optionally invalidate cache if mode changed
    pub fn set_album_aware(&mut self, album_aware: bool) {
        if self.album_aware != album_aware {
            // Mode changed, invalidate cache to force refresh
            self.invalidate_cache();
            println!("[+] Album-aware mode changed, cache invalidated");
        }
        self.album_aware = album_aware;
    }

    // Call this when tags are updated to invalidate cache
    pub fn invalidate_cache(&mut self) {
        self.regular_cache = None;
        self.album_cache = None;
        self.cache_timestamp = None;
        println!("[+] Song cache invalidated");
    }

    fn is_cache_valid(&self) -> bool {
        if let Some(timestamp) = self.cache_timestamp {
            timestamp.elapsed() < self.cache_ttl
        } else {
            false
        }
    }

    // Add a song to the queue
    pub fn add(&mut self, song: mpd::Song) {
        self.inner.push_back(song);
    }

    // Remove and return the next song from the queue
    pub fn remove(&mut self) -> Option<mpd::Song> {
        self.inner.pop_front()
    }

    // Peek at the next song in the queue
    #[allow(dead_code)]
    pub fn peek(&self) -> Option<&mpd::Song> {
        self.inner.front()
    }

    // Get the length of the queue
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    // Get a slice of the first n songs in the queue, defaulting to 3 if count is None
    pub fn head(&self, count: Option<usize>) -> Vec<mpd::Song> {
        let count = count.unwrap_or(3);
        self.inner.iter().take(count).cloned().collect()
    }

    // Get a slice of the last n songs in the queue, defaulting to 3 if count is None
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

    // NEW: Main shuffle method that uses caching intelligently
    pub fn shuffle_and_add_with_cache(
        &mut self, 
        tags_data: &crate::models::tags_data::TagsData, 
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn
    ) {
        let start_time = Instant::now();
        
        // Get songs from appropriate cache or fetch fresh
        let songs = if self.album_aware {
            if let Some(ref cached) = self.album_cache {
                if self.is_cache_valid() {
                    println!("[+] Using cached album-aware songs ({} songs)", cached.len());
                    cached.clone()
                } else {
                    println!("[+] Album cache expired, refreshing...");
                    let fresh_songs = tags_data.get_album_aware_songs(mpd_client);
                    self.album_cache = Some(fresh_songs.clone());
                    self.cache_timestamp = Some(Instant::now());
                    fresh_songs
                }
            } else {
                println!("[+] Building album-aware cache...");
                let fresh_songs = tags_data.get_album_aware_songs(mpd_client);
                self.album_cache = Some(fresh_songs.clone());
                self.cache_timestamp = Some(Instant::now());
                fresh_songs
            }
        } else {
            if let Some(ref cached) = self.regular_cache {
                if self.is_cache_valid() {
                    println!("[+] Using cached regular songs ({} songs)", cached.len());
                    cached.clone()
                } else {
                    println!("[+] Regular cache expired, refreshing...");
                    let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                    self.regular_cache = Some(fresh_songs.clone());
                    self.cache_timestamp = Some(Instant::now());
                    fresh_songs
                }
            } else {
                println!("[+] Building regular cache...");
                let fresh_songs = tags_data.get_allowed_songs(mpd_client);
                self.regular_cache = Some(fresh_songs.clone());
                self.cache_timestamp = Some(Instant::now());
                fresh_songs
            }
        };

        // Use existing shuffle logic
        if self.album_aware {
            self.shuffle_and_add_album_aware(songs);
        } else {
            self.shuffle_and_add_regular(songs);
        }

        println!("[+] Cached shuffle_and_add took: {:?}", start_time.elapsed());
    }

    // Keep the old method for backward compatibility
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
        let mut rng = rand::rng(); // Fixed: use rand::rng() instead of thread_rng()
        song_vec.shuffle(&mut rng);

        for song in song_vec {
            self.add(song);
        }

        let elapsed_time = start_time.elapsed();
        println!("[-] shuffle_and_add_regular took: {:?}", elapsed_time);
    }

    // Helper function to get a tag value from a song
    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags.iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    fn shuffle_and_add_album_aware(&mut self, songs: HashSet<HashableSong>) {
        let start_time = std::time::Instant::now();
        self.empty_queue();

        let mut albums: HashMap<String, Vec<mpd::Song>> = HashMap::new();
        
        for hashable_song in songs {
            let song = mpd::Song::from(hashable_song);
            let album_name = Self::get_tag_value(&song, "Album")
                .unwrap_or_else(|| "Unknown Album".to_string());
            
            albums.entry(album_name).or_insert_with(Vec::new).push(song);
        }

        // Sort each album by track number
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
            
            println!("[+] sorted album '{}' with {} tracks", album_name, album_songs.len());
        }

        // Shuffle the order of albums
        let mut album_names: Vec<String> = albums.keys().cloned().collect();
        let mut rng = rand::rng(); // Fixed: use rand::rng() instead of thread_rng()
        album_names.shuffle(&mut rng);

        // Add albums in shuffled order, but songs within each album in track order
        for album_name in album_names {
            if let Some(album_songs) = albums.remove(&album_name) {
                println!("[+] adding album '{}' to queue", album_name);
                for song in album_songs {
                    self.add(song);
                }
            }
        }

        let elapsed_time = start_time.elapsed();
        println!("[-] album-aware shuffle_and_add took: {:?}", elapsed_time);
    }

    #[allow(dead_code)]
    pub fn shuffle(&mut self) {
        let start_time = std::time::Instant::now();
        let mut rng = rand::rng(); // Fixed: use rand::rng() instead of thread_rng()
        let mut vec: Vec<mpd::Song> = self.inner.drain(..).collect();
        vec.shuffle(&mut rng);
        self.inner.extend(vec);
        let elapsed_time = start_time.elapsed();
        println!("[-] shuffle took: {:?}", elapsed_time);
    }

    // Optional: Add cache status methods for debugging
    #[allow(dead_code)]
    pub fn has_regular_cache(&self) -> bool {
        self.regular_cache.is_some()
    }

    #[allow(dead_code)]
    pub fn has_album_cache(&self) -> bool {
        self.album_cache.is_some()
    }

    #[allow(dead_code)]
    pub fn cache_age(&self) -> Option<Duration> {
        self.cache_timestamp.map(|t| t.elapsed())
    }
}
