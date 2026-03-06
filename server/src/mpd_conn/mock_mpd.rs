use crate::mpd_conn::traits::MpdClient;
use mpd::{error::Error, error::ErrorCode, error::Result, error::ServerError, Playlist, Query, Song};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockMpd {
    playlists: Arc<Mutex<HashMap<String, Vec<Song>>>>,
    queue: Arc<Mutex<Vec<Song>>>,
    pushed_history: Arc<Mutex<Vec<Song>>>,
    is_consuming: Arc<Mutex<bool>>,
    connection_state: Arc<Mutex<bool>>, // true if connected
}

impl MockMpd {
    pub fn new() -> Self {
        MockMpd {
            playlists: Arc::new(Mutex::new(HashMap::new())),
            queue: Arc::new(Mutex::new(Vec::new())),
            pushed_history: Arc::new(Mutex::new(Vec::new())),
            is_consuming: Arc::new(Mutex::new(false)),
            connection_state: Arc::new(Mutex::new(true)),
        }
    }

    // Add a new playlist or replace an existing one
    pub fn add_playlist(&self, name: &str, songs: Vec<Song>) {
        let mut playlists = self.playlists.lock().unwrap();
        playlists.insert(name.to_string(), songs);
    }

    // Simulate disconnection
    pub fn simulate_disconnect(&self) {
        let mut state = self.connection_state.lock().unwrap();
        *state = false;
    }

    // Simulate reconnection
    pub fn simulate_reconnect(&self) {
        let mut state = self.connection_state.lock().unwrap();
        *state = true;
    }

    // Clear all internal state
    pub fn clear_state(&self) {
        self.playlists.lock().unwrap().clear();
        self.queue.lock().unwrap().clear();
        self.pushed_history.lock().unwrap().clear();
        *self.is_consuming.lock().unwrap() = false;
        *self.connection_state.lock().unwrap() = true;
    }

    pub fn get_pushed_history(&self) -> Vec<Song> {
        self.pushed_history.lock().unwrap().clone()
    }

    fn check_connection(&self) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }
        Ok(())
    }
}

// Implement MpdClient trait for MockMpd
impl MpdClient for MockMpd {
    fn ping(&mut self) -> Result<()> {
        self.check_connection()
    }

    fn playlist(&mut self, name: &str) -> Result<Vec<Song>> {
        self.check_connection()?;
        let playlists = self.playlists.lock().unwrap();
        match playlists.get(name) {
            Some(songs) => Ok(songs.clone()),
            None => Err(Error::Server(ServerError {
                code: ErrorCode::NoExist,
                pos: 0,
                command: "playlist".to_string(),
                detail: "Playlist not found".to_string(),
            })),
        }
    }

    fn playlists(&mut self) -> Result<Vec<Playlist>> {
        self.check_connection()?;
        let playlists = self.playlists.lock().unwrap();
        Ok(playlists
            .keys()
            .map(|name| Playlist {
                name: name.clone(),
                last_mod: "".to_string(),
            })
            .collect())
    }

    fn queue(&mut self) -> Result<Vec<Song>> {
        self.check_connection()?;
        let queue = self.queue.lock().unwrap();
        Ok(queue.clone())
    }

    fn search(&mut self, _query: &Query, _window: Option<(u32, u32)>) -> Result<Vec<Song>> {
        self.check_connection()?;
        // Simple mock search returns all songs from all playlists for now
        // A more advanced mock could actually interpret the query
        let playlists = self.playlists.lock().unwrap();
        let mut all_songs = Vec::new();
        for playlist in playlists.values() {
            all_songs.extend(playlist.clone());
        }
        Ok(all_songs)
    }

    fn consume(&mut self, state: bool) -> Result<()> {
        self.check_connection()?;
        let mut consume_state = self.is_consuming.lock().unwrap();
        *consume_state = state;
        Ok(())
    }

    fn push(&mut self, song: Song) -> Result<mpd::Id> {
        self.check_connection()?;
        let mut queue = self.queue.lock().unwrap();
        queue.push(song.clone());
        self.pushed_history.lock().unwrap().push(song);
        Ok(mpd::Id(queue.len() as u32))
    }

    fn delete(&mut self, pos: u32) -> Result<()> {
        self.check_connection()?;
        let mut queue = self.queue.lock().unwrap();
        if pos as usize >= queue.len() {
            return Err(Error::Server(ServerError {
                code: ErrorCode::NoExist,
                pos: 0,
                command: "delete".to_string(),
                detail: "Position out of bounds".to_string(),
            }));
        }
        queue.remove(pos as usize);
        Ok(())
    }

    fn play(&mut self) -> Result<()> {
        self.check_connection()
    }

    fn pl_push(&mut self, playlist_name: &str, song: Song) -> Result<()> {
        self.check_connection()?;
        let mut playlists = self.playlists.lock().unwrap();
        playlists
            .entry(playlist_name.to_string())
            .or_insert_with(Vec::new)
            .push(song);
        Ok(())
    }

    fn pl_delete(&mut self, playlist_name: &str, pos: u32) -> Result<()> {
        self.check_connection()?;
        let mut playlists = self.playlists.lock().unwrap();
        if let Some(playlist) = playlists.get_mut(playlist_name) {
            if pos as usize >= playlist.len() {
                return Err(Error::Server(ServerError {
                    code: ErrorCode::NoExist,
                    pos: 0,
                    command: "delete".to_string(),
                    detail: "Position out of bounds".to_string(),
                }));
            }
            playlist.remove(pos as usize);
            Ok(())
        } else {
            Err(Error::Server(ServerError {
                code: ErrorCode::NoExist,
                pos: 0,
                command: "pl_delete".to_string(),
                detail: format!("Playlist {} not found", playlist_name),
            }))
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
