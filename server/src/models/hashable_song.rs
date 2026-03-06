use crate::mpd_conn::traits::Song;
use std::hash::{Hash, Hasher};

/// A wrapper around `Song` that enables hashing and equality
/// based on the song's file path.
#[derive(Debug, Clone)]
pub struct HashableSong(pub Song);

impl PartialEq for HashableSong {
    fn eq(&self, other: &Self) -> bool {
        self.0.file == other.0.file
    }
}

impl Eq for HashableSong {}

impl Hash for HashableSong {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.file.hash(state);
    }
}

impl From<HashableSong> for Song {
    fn from(hashable_song: HashableSong) -> Self {
        hashable_song.0
    }
}

impl From<Song> for HashableSong {
    fn from(song: Song) -> Self {
        HashableSong(song)
    }
}
