use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::models::hashable_song::HashableSong;

// Cache entry with timestamp and tag hash
#[derive(Clone)]
struct CacheEntry {
    songs: HashSet<HashableSong>,
    timestamp: Instant,
    tags_hash: u64,
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

pub struct SongQueue {
    inner: VecDeque<mpd::Song>,
    album_aware: bool,

    // Simple cache: stores the result of get_allowed_songs()
    // This is valuable because querying MPD playlists is slow
    cache: Option<CacheEntry>,
    cache_ttl: Duration,

    // Performance tracking
    cache_hits: u64,
    cache_misses: u64,
}

impl SongQueue {
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            album_aware: false,
            cache: None,
            cache_ttl: Duration::from_secs(600), // 10 minute cache
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    pub fn set_album_aware(&mut self, album_aware: bool) {
        if self.album_aware != album_aware {
            println!("[+] Album-aware mode changed to {}", album_aware);
        }
        self.album_aware = album_aware;
    }

    pub fn invalidate_cache(&mut self) {
        self.cache = None;
        println!("[+] Cache invalidated (tags changed)");
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

    // Main method: get songs (from cache if valid, otherwise query MPD)
    fn get_or_fetch_songs(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) -> HashSet<HashableSong> {
        let tags_hash = Self::hash_tags(tags_data);

        // Check if we have a valid cache
        if let Some(ref cache) = self.cache {
            if cache.is_valid(tags_hash, self.cache_ttl) {
                self.cache_hits += 1;
                println!("[+] CACHE HIT: Using {} cached songs", cache.songs.len());
                return cache.songs.clone();
            }
        }

        // Cache miss - query MPD
        self.cache_misses += 1;
        println!("[+] CACHE MISS: Querying MPD for songs...");

        let start = Instant::now();
        let songs = tags_data.get_allowed_songs(mpd_client);
        let query_time = start.elapsed();

        println!(
            "[+] MPD query took {:?}, found {} songs",
            query_time,
            songs.len()
        );

        // Update cache
        self.cache = Some(CacheEntry::new(songs.clone(), tags_hash));

        songs
    }

    // Main shuffle method - now simpler!
    pub fn shuffle_and_add(
        &mut self,
        tags_data: &crate::models::tags_data::TagsData,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) {
        let start_time = Instant::now();

        // Get songs (cached or fresh)
        let songs = self.get_or_fetch_songs(tags_data, mpd_client);

        // Shuffle and load into queue
        self.inner.clear();
        self.inner.reserve(songs.len());

        let mut song_vec: Vec<mpd::Song> = songs.into_iter().map(mpd::Song::from).collect();
        let mut rng = rand::rng();
        song_vec.shuffle(&mut rng);

        for song in song_vec {
            self.inner.push_back(song);
        }

        println!(
            "[+] Shuffle complete: {} songs loaded in {:?}",
            self.inner.len(),
            start_time.elapsed()
        );
    }

    pub fn add(&mut self, song: mpd::Song) {
        self.inner.push_back(song);
    }

    /// Dequeue songs based on the current mode
    /// - In regular mode: returns a single song
    /// - In album-aware mode: returns all songs from the album
    pub fn dequeue(
        &mut self,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) -> Vec<mpd::Song> {
        if self.album_aware {
            self.dequeue_as_album(mpd_client)
        } else {
            self.dequeue_single()
        }
    }

    /// Dequeue a single song (regular mode)
    /// Made public for testing purposes
    pub fn dequeue_single(&mut self) -> Vec<mpd::Song> {
        self.inner.pop_front().map(|s| vec![s]).unwrap_or_default()
    }

    /// Remove a single song from the queue (for backward compatibility in tests)
    /// Returns Option<Song> to match old API
    pub fn remove(&mut self) -> Option<mpd::Song> {
        self.inner.pop_front()
    }

    /// Dequeue a full album (album-aware mode)
    /// Takes the next song from the queue and returns all songs from its album
    fn dequeue_as_album(
        &mut self,
        mpd_client: &mut crate::mpd_conn::mpd_conn::MpdConn,
    ) -> Vec<mpd::Song> {
        let seed_song = match self.inner.pop_front() {
            Some(song) => song,
            None => return Vec::new(),
        };

        let album_name = Self::get_tag_value(&seed_song, "Album")
            .unwrap_or_else(|| "Unknown Album".to_string());

        println!("[+] Album-aware: Loading full album '{}'", album_name);

        // Query MPD for all songs from this album
        let album_songs = {
            let mut query = mpd::Query::new();
            query.and(mpd::Term::Tag("album".into()), album_name.as_str());

            match mpd_client.mpd.search(&query, None) {
                Ok(songs) => songs,
                Err(e) => {
                    eprintln!("[-] Error querying album: {}", e);
                    return vec![seed_song];
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

        sorted_songs
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

    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags
            .iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    // Performance monitoring
    pub fn cache_stats(&self) -> (u64, u64, f64) {
        let total = self.cache_hits + self.cache_misses;
        let hit_rate = if total > 0 {
            (self.cache_hits as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        (self.cache_hits, self.cache_misses, hit_rate)
    }

    pub fn has_valid_cache(&self, tags_data: &crate::models::tags_data::TagsData) -> bool {
        let tags_hash = Self::hash_tags(tags_data);
        self.cache
            .as_ref()
            .map_or(false, |c| c.is_valid(tags_hash, self.cache_ttl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_song(path: &str) -> mpd::Song {
        let mut song = mpd::Song::default();
        song.file = path.to_string();
        song
    }

    #[test]
    fn test_dequeue_modes() {
        let mut queue = SongQueue::new();
        
        // Add test songs
        queue.add(create_test_song("song1.mp3"));
        queue.add(create_test_song("song2.mp3"));
        
        // In regular mode, dequeue returns one song
        queue.set_album_aware(false);
        assert_eq!(queue.len(), 2);
        
        let songs = queue.dequeue_single();
        assert_eq!(songs.len(), 1);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_album_aware_flag() {
        let mut queue = SongQueue::new();
        
        queue.set_album_aware(true);
        // Can't easily test dequeue_as_album without real MPD
        // but we can verify the flag is set
        assert!(queue.album_aware);
        
        queue.set_album_aware(false);
        assert!(!queue.album_aware);
    }
}
