use rand::seq::SliceRandom;
use std::collections::{HashMap, HashSet};
use std::collections::VecDeque;

use crate::models::hashable_song::HashableSong;

// Define your custom queue type
pub struct SongQueue {
    inner: VecDeque<mpd::Song>,
    album_aware: bool,
}

impl SongQueue {
    // Initialize a new SongQueue
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
            album_aware: false,
        }
    }

    // Set album-aware mode
    pub fn set_album_aware(&mut self, album_aware: bool) {
        self.album_aware = album_aware;
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

    pub fn shuffle_and_add(&mut self, songs: HashSet<HashableSong>) {
        if self.album_aware {
            self.shuffle_and_add_album_aware(songs);
        } else {
            self.shuffle_and_add_regular(songs);
        }
    }

    fn shuffle_and_add_regular(&mut self, songs: HashSet<HashableSong>) {
        // Start the timer
        let start_time = std::time::Instant::now();

        // Reserve space in the VecDeque for the songs
        self.inner.reserve(songs.len());

        // Empty the queue
        self.empty_queue();

        // Convert HashSet to Vec for shuffling
        let mut song_vec: Vec<mpd::Song> = songs.into_iter().map(mpd::Song::from).collect();
        let mut rng = rand::rng();
        song_vec.shuffle(&mut rng);

        for song in song_vec {
            self.add(song);
        }

        // Stop the timer
        let elapsed_time = start_time.elapsed();
        println!("[-] shuffle_and_add took: {:?}", elapsed_time);
    }

    // Helper function to get a tag value from a song
    fn get_tag_value(song: &mpd::Song, tag_name: &str) -> Option<String> {
        song.tags.iter()
            .find(|(key, _)| key == tag_name)
            .map(|(_, value)| value.clone())
    }

    fn shuffle_and_add_album_aware(&mut self, songs: HashSet<HashableSong>) {
        // Start the timer
        let start_time = std::time::Instant::now();

        // Empty the queue
        self.empty_queue();

        // Group songs by album
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
        let mut rng = rand::rng();
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

        // Stop the timer
        let elapsed_time = start_time.elapsed();
        println!("[-] album-aware shuffle_and_add took: {:?}", elapsed_time);
    }

    #[allow(dead_code)]
    pub fn shuffle(&mut self) {
        // Start the timer
        let start_time = std::time::Instant::now();

        let mut rng = rand::rng();
        let mut vec: Vec<mpd::Song> = self.inner.drain(..).collect();
        vec.shuffle(&mut rng);
        self.inner.extend(vec);

        // Stop the timer
        let elapsed_time = start_time.elapsed();
        println!("[-] shuffle took: {:?}", elapsed_time);
    }
}
