use std::hash::{Hash, Hasher};

// Create a newtype wrapper for Mpd::Song
pub struct HashableSong(pub mpd::Song);

impl Eq for HashableSong {}

impl Hash for HashableSong {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the filename field for uniqueness
        self.0.file.hash(state);
    }
}

impl PartialEq for HashableSong {
    fn eq(&self, other: &Self) -> bool {
        // Compare based on the filename field
        self.0.file == other.0.file
    }
}

impl From<HashableSong> for mpd::Song {
    fn from(hashable_song: HashableSong) -> Self {
        hashable_song.0 // Extract the inner mpd::Song
    }
}
