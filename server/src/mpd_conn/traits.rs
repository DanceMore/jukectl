use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Song {
    pub file: String,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<u32>,
    pub pos: Option<u32>,
    pub id: Option<u32>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FilterTerm {
    Any(String),
    Tag(String, String),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Query {
    pub terms: Vec<FilterTerm>,
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn and(&mut self, term: FilterTerm) -> &mut Self {
        self.terms.push(term);
        self
    }
}

pub trait MpdClient: Send {
    fn ping(&mut self) -> Result<()>;
    fn playlist(&mut self, name: &str) -> Result<Vec<Song>>;
    fn playlists(&mut self) -> Result<Vec<Playlist>>;
    fn queue(&mut self) -> Result<Vec<Song>>;
    fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<Song>>;
    fn consume(&mut self, state: bool) -> Result<()>;
    fn push(&mut self, file: &str) -> Result<u32>;
    fn delete(&mut self, pos: u32) -> Result<()>;
    fn play(&mut self) -> Result<()>;
    fn pl_push(&mut self, playlist: &str, file: &str) -> Result<()>;
    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()>;
    fn pl_remove(&mut self, playlist: &str) -> Result<()>;
    fn listall(&mut self) -> Result<Vec<Song>>;
}
