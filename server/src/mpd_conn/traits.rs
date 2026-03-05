use mpd::error::Result;
use mpd::{Playlist, Query, Song};

/// Trait defining the operations we need from an MPD client.
/// This allows us to switch between a real MPD connection and a mock.
pub trait MpdClient: Send + Sync {
    fn ping(&mut self) -> Result<()>;
    fn playlist(&mut self, name: &str) -> Result<Vec<Song>>;
    fn playlists(&mut self) -> Result<Vec<Playlist>>;
    fn queue(&mut self) -> Result<Vec<Song>>;
    fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<Song>>;
    fn consume(&mut self, state: bool) -> Result<()>;
    fn push(&mut self, song: Song) -> Result<mpd::Id>;
    fn delete(&mut self, pos: u32) -> Result<()>;
    fn play(&mut self) -> Result<()>;
    fn pl_push(&mut self, playlist: &str, song: Song) -> Result<()>;
    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()>;
    fn pl_remove(&mut self, playlist: &str) -> Result<()>;
    fn listall(&mut self) -> Result<Vec<Song>>;
}

/// Implement MpdClient for the real mpd::Client
impl MpdClient for mpd::Client {
    fn ping(&mut self) -> Result<()> {
        self.ping()
    }

    fn playlist(&mut self, name: &str) -> Result<Vec<Song>> {
        self.playlist(name)
    }

    fn playlists(&mut self) -> Result<Vec<Playlist>> {
        self.playlists()
    }

    fn queue(&mut self) -> Result<Vec<Song>> {
        self.queue()
    }

    fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<Song>> {
        self.search(query, window)
    }

    fn consume(&mut self, state: bool) -> Result<()> {
        self.consume(state)
    }

    fn push(&mut self, song: Song) -> Result<mpd::Id> {
        self.push(song)
    }

    fn delete(&mut self, pos: u32) -> Result<()> {
        self.delete(pos)
    }

    fn play(&mut self) -> Result<()> {
        self.play()
    }

    fn pl_push(&mut self, playlist: &str, song: Song) -> Result<()> {
        self.pl_push(playlist, song)
    }

    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()> {
        self.pl_delete(playlist, pos)
    }

    fn pl_remove(&mut self, playlist: &str) -> Result<()> {
        self.pl_remove(playlist)
    }

    fn listall(&mut self) -> Result<Vec<Song>> {
        self.listall()
    }
}
