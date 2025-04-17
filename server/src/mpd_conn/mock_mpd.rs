use mpd::{
    error::Error, error::ErrorCode, error::Result, error::ServerError, Client, Playlist, Song,
};
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
}

// Implement common MPD client methods for MockMpd
impl MockMpd {
    pub fn playlist(&self, name: &str) -> Result<Vec<Song>> {
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

    pub fn queue(&self) -> Result<Vec<Song>> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        let queue = self.queue.lock().unwrap();
        Ok(queue.clone())
    }

    pub fn push(&self, song: Song) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        let mut queue = self.queue.lock().unwrap();
        queue.push(song);
        Ok(())
    }

    pub fn delete(&self, pos: u32) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

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

    pub fn consume(&self, state: bool) -> Result<()> {
        let connection_state = self.connection_state.lock().unwrap();
        if !*connection_state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        let mut consume_state = self.is_consuming.lock().unwrap();
        *consume_state = state;
        Ok(())
    }

    pub fn play(&self) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        Ok(())
    }

    pub fn ping(&self) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        Ok(())
    }

    pub fn pl_push(&self, playlist_name: &str, song: Song) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

        let mut playlists = self.playlists.lock().unwrap();

        // Create playlist if it doesn't exist
        if !playlists.contains_key(playlist_name) {
            playlists.insert(playlist_name.to_string(), Vec::new());
        }

        // Add song to playlist
        if let Some(playlist) = playlists.get_mut(playlist_name) {
            playlist.push(song);
            Ok(())
        } else {
            Err(Error::Server(ServerError {
                code: ErrorCode::NoExist,
                pos: 0,
                command: "pl_push".to_string(),
                detail: "Failed to add song to plalist".to_string(),
            }))
        }
    }

    pub fn pl_delete(&self, playlist_name: &str, pos: u32) -> Result<()> {
        let state = self.connection_state.lock().unwrap();
        if !*state {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Not connected",
            )));
        }

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
            let error_msg = format!("Playlist {} not found", playlist_name);
            Err(Error::Server(ServerError {
                code: ErrorCode::NoExist,
                pos: 0,
                command: "pl_delete".to_string(),
                detail: error_msg,
            }))
        }
    }
}
