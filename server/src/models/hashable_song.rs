/// A wrapper around `mpd::Song` that enables hashing and equality
/// comparison based on the song's file path.
///
/// we care about song identity based on the unique `file` field as
/// our "Primary Key" , and not the full metadata (e.g., artist, title, duration).
///
/// For example, this is used to construct playlists based on tags,
/// ensuring no duplicates even when the same song appears under multiple tags.
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
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
