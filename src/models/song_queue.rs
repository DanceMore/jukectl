use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::collections::VecDeque;

use crate::HashableSong;

// Define your custom queue type
pub struct SongQueue {
    inner: VecDeque<mpd::Song>, // Use mpd::Song as the element type
}

impl SongQueue {
    // Initialize a new SongQueue
    pub fn new() -> Self {
        Self {
            inner: VecDeque::new(),
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

    // Get a slice of the first 3 songs in the queue
    pub fn head(&self) -> Vec<&mpd::Song> {
        self.inner.iter().take(3).collect()
    }

    // Get a slice of the last 3 songs in the queue
    pub fn tail(&self) -> Vec<&mpd::Song> {
        let len = self.inner.len();
        self.inner.iter().skip(len.saturating_sub(3)).collect()
    }

    pub fn shuffle(&mut self) {
        // Start the timer
        let start_time = std::time::Instant::now();

        let mut rng = rand::thread_rng();
        let mut vec: Vec<mpd::Song> = self.inner.drain(..).collect();
        vec.shuffle(&mut rng);
        self.inner.extend(vec);

        // Stop the timer
        let elapsed_time = start_time.elapsed();
        println!("shuffle took: {:?}", elapsed_time);
    }

    pub fn empty_queue(&mut self) {
        self.inner.clear();
    }

    pub fn shuffle_and_add(&mut self, songs: HashSet<HashableSong>) {
        // Start the timer
        let start_time = std::time::Instant::now();

        // Reserve space in the VecDeque for the songs
        self.inner.reserve(songs.len());

        // Empty the queue
        self.empty_queue();

        // Convert HashSet to Vec for shuffling
        let mut song_vec: Vec<mpd::Song> = songs
            .into_iter()
            .map(|song| mpd::Song::from(song))
            .collect();
        let mut rng = rand::thread_rng();
        song_vec.shuffle(&mut rng);

        for song in song_vec {
            self.add(song);
        }

        // Stop the timer
        let elapsed_time = start_time.elapsed();
        println!("shuffle_and_add took: {:?}", elapsed_time);
    }
}
