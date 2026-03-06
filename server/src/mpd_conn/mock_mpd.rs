use crate::mpd_conn::traits::{FilterTerm, MpdClient, Playlist, Query, Song};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockMpd {
    playlists: Arc<Mutex<HashMap<String, Vec<Song>>>>,
    queue: Arc<Mutex<Vec<Song>>>,
    is_consuming: Arc<Mutex<bool>>,
    connection_state: Arc<Mutex<bool>>, // true if connected
}

impl MockMpd {
    pub fn new() -> Self {
        MockMpd {
            playlists: Arc::new(Mutex::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            is_consuming: Arc::new(Mutex::new(false)),
            connection_state: Arc::new(Mutex::new(true)),
        }
    }

    pub fn add_playlist(&self, name: &str, songs: Vec<Song>) {
        let mut playlists = self.playlists.lock().unwrap();
        playlists.insert(name.to_string(), songs);
    }

    pub fn simulate_disconnect(&self) {
        let mut state = self.connection_state.lock().unwrap();
        *state = false;
    }

    pub fn simulate_reconnect(&self) {
        let mut state = self.connection_state.lock().unwrap();
        *state = true;
    }

    fn check_connection(&self) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(anyhow!("Not connected"));
        }
        Ok(())
    }
}

impl MpdClient for MockMpd {
    fn ping(&mut self) -> Result<()> {
        self.check_connection()
    }

    fn playlist(&mut self, name: &str) -> Result<Vec<Song>> {
        self.check_connection()?;
        let playlists = self.playlists.lock().unwrap();
        match playlists.get(name) {
            Some(songs) => Ok(songs.clone()),
            None => Err(anyhow!("Playlist not found")),
        }
    }

    fn playlists(&mut self) -> Result<Vec<Playlist>> {
        self.check_connection()?;
        let playlists = self.playlists.lock().unwrap();
        Ok(playlists
            .keys()
            .map(|name| Playlist { name: name.clone() })
            .collect())
    }

    fn queue(&mut self) -> Result<Vec<Song>> {
        self.check_connection()?;
        let queue = self.queue.lock().unwrap();
        Ok(queue.clone())
    }

    fn search(&mut self, query: &Query, _window: Option<(u32, u32)>) -> Result<Vec<Song>> {
        self.check_connection()?;

        let playlists = self.playlists.lock().unwrap();
        let mut all_songs = Vec::new();
        for playlist in playlists.values() {
            all_songs.extend(playlist.clone());
        }

        let filtered_songs = all_songs
            .into_iter()
            .filter(|song| {
                query.terms.iter().all(|term| match term {
                    FilterTerm::Any(val) => {
                        song.file.contains(val)
                            || song.title.as_ref().map_or(false, |t| t.contains(val))
                            || song.artist.as_ref().map_or(false, |a| a.contains(val))
                            || song.album.as_ref().map_or(false, |a| a.contains(val))
                    }
                    FilterTerm::Tag(tag, val) => match tag.to_lowercase().as_str() {
                        "artist" => song.artist.as_ref().map_or(false, |a| a == val),
                        "album" => song.album.as_ref().map_or(false, |a| a == val),
                        "title" => song.title.as_ref().map_or(false, |t| t == val),
                        _ => false,
                    },
                })
            })
            .collect();

        Ok(filtered_songs)
    }

    fn consume(&mut self, state: bool) -> Result<()> {
        self.check_connection()?;
        let mut consume_state = self.is_consuming.lock().unwrap();
        *consume_state = state;
        Ok(())
    }

    fn push(&mut self, file: &str) -> Result<u32> {
        self.check_connection()?;
        let mut queue = self.queue.lock().unwrap();
        let id = queue.len() as u32;
        queue.push(Song {
            file: file.to_string(),
            title: None,
            artist: None,
            album: None,
            duration: None,
            pos: Some(id),
            id: Some(id),
        });
        Ok(id)
    }

    fn delete(&mut self, pos: u32) -> Result<()> {
        self.check_connection()?;
        let mut queue = self.queue.lock().unwrap();
        if pos as usize >= queue.len() {
            return Err(anyhow!("Position out of bounds"));
        }
        queue.remove(pos as usize);
        Ok(())
    }

    fn play(&mut self) -> Result<()> {
        self.check_connection()
    }

    fn pl_push(&mut self, playlist_name: &str, file: &str) -> Result<()> {
        self.check_connection()?;
        let mut playlists = self.playlists.lock().unwrap();
        playlists
            .entry(playlist_name.to_string())
            .or_insert_with(Vec::new)
            .push(Song {
                file: file.to_string(),
                title: None,
                artist: None,
                album: None,
                duration: None,
                pos: None,
                id: None,
            });
        Ok(())
    }

    fn pl_delete(&mut self, playlist_name: &str, pos: u32) -> Result<()> {
        self.check_connection()?;
        let mut playlists = self.playlists.lock().unwrap();
        if let Some(playlist) = playlists.get_mut(playlist_name) {
            if pos as usize >= playlist.len() {
                return Err(anyhow!("Position out of bounds"));
            }
            playlist.remove(pos as usize);
            Ok(())
        } else {
            Err(anyhow!("Playlist {} not found", playlist_name))
        }
    }

    fn pl_remove(&mut self, playlist: &str) -> Result<()> {
        self.check_connection()?;
        let mut playlists = self.playlists.lock().unwrap();
        playlists.remove(playlist);
        Ok(())
    }

    fn listall(&mut self) -> Result<Vec<Song>> {
        self.check_connection()?;
        let playlists = self.playlists.lock().unwrap();
        let mut all_songs = Vec::new();
        for playlist in playlists.values() {
            all_songs.extend(playlist.clone());
        }
        Ok(all_songs)
    }
}
