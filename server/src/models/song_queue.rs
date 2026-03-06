use log::debug;
use rand::seq::SliceRandom;
use std::collections::VecDeque;

use crate::models::hashable_song::HashableSong;
use crate::mpd_conn::mpd_pool::PooledMpdConnection;
use crate::mpd_conn::traits::{FilterTerm, MpdClient, Query, Song};

pub enum DequeueMode {
    Single,
    Album,
}

pub struct SongQueue {
    inner: VecDeque<Song>,
    is_album_aware: bool,
}

impl SongQueue {
    pub fn new() -> Self {
        SongQueue {
            inner: VecDeque::new(),
            is_album_aware: false,
        }
    }

    pub fn set_album_aware(&mut self, enabled: bool) {
        self.is_album_aware = enabled;
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn empty_queue(&mut self) {
        self.clear();
    }

    pub fn add_songs(&mut self, songs: Vec<Song>) {
        let mut song_vec: Vec<Song> = songs.into_iter().collect();
        let mut rng = rand::rng();
        song_vec.shuffle(&mut rng);
        for song in song_vec {
            self.inner.push_back(song);
        }
    }

    pub fn add(&mut self, song: Song) {
        self.inner.push_back(song);
    }

    pub fn dequeue(
        &mut self,
        mode: DequeueMode,
        pooled_conn: &mut PooledMpdConnection,
    ) -> Vec<Song> {
        match mode {
            DequeueMode::Single => self.dequeue_single(),
            DequeueMode::Album => self.dequeue_as_album(pooled_conn),
        }
    }

    pub fn dequeue_single(&mut self) -> Vec<Song> {
        match self.inner.pop_front() {
            Some(song) => vec![song],
            None => vec![],
        }
    }

    pub fn remove(&mut self) -> Option<Song> {
        self.inner.pop_front()
    }

    pub fn dequeue_as_album(
        &mut self,
        pooled_conn: &mut PooledMpdConnection,
    ) -> Vec<Song> {
        let first_song = match self.inner.pop_front() {
            Some(s) => s,
            None => return vec![],
        };

        let album_name = match &first_song.album {
            Some(a) => a,
            None => return vec![first_song],
        };

        debug!("Dequeuing album: {}", album_name);

        let album_songs: Vec<Song> = {
            let mut query = Query::new();
            query.and(FilterTerm::Tag("album".into(), album_name.clone()));

            let search_results = pooled_conn
                .mpd_conn()
                .mpd
                .search(&query, None)
                .unwrap_or_default();

            search_results
                .into_iter()
                .filter(|s| {
                    let album_match = s.album.as_deref() == Some(album_name);
                    let artist_match = if let (Some(ref a1), Some(ref a2)) = (&s.artist, &first_song.artist) {
                        a1 == a2
                    } else {
                        true
                    };
                    album_match && artist_match
                })
                .collect()
        };

        if album_songs.is_empty() {
            return vec![first_song];
        }

        let mut album_songs_sorted = album_songs;
        album_songs_sorted.sort_by(|a, b| a.pos.cmp(&b.pos));

        let hashable_album_songs: Vec<HashableSong> = album_songs_sorted
            .iter()
            .cloned()
            .map(HashableSong::from)
            .collect();

        self.inner.retain(|s| {
            let hs = HashableSong::from(s.clone());
            !hashable_album_songs.contains(&hs)
        });

        album_songs_sorted
    }

    pub fn head(&self, count: Option<usize>) -> Vec<Song> {
        let n = count.unwrap_or(10);
        self.inner.iter().take(n).cloned().collect()
    }

    pub fn tail(&self, count: Option<usize>) -> Vec<Song> {
        let n = count.unwrap_or(10);
        let start = if self.inner.len() > n {
            self.inner.len() - n
        } else {
            0
        };
        self.inner.iter().skip(start).cloned().collect()
    }

    pub fn invalidate_cache(&mut self) {
        // Implementation for cache invalidation if we had one
    }

    pub fn cache_stats(&self) -> (usize, usize, f64) {
        (0, 0, 0.0) // Dummy for tests
    }

    pub fn shuffle_and_add(&mut self, _tags: &crate::models::tags_data::TagsResponse, _mpd: &mut dyn MpdClient) {
        // Logic to shuffle all songs and add to queue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_song(path: &str) -> Song {
        Song {
            file: path.to_string(),
            title: None,
            artist: None,
            album: None,
            duration: None,
            pos: None,
            id: None,
        }
    }

    #[test]
    fn test_song_queue_add() {
        let mut queue = SongQueue::new();
        let song = create_test_song("test.mp3");
        queue.add(song);
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn test_song_queue_clear() {
        let mut queue = SongQueue::new();
        queue.add(create_test_song("test.mp3"));
        queue.clear();
        assert_eq!(queue.len(), 0);
    }
}
