use jukectl_server::models::hashable_song::HashableSong;

#[cfg(test)]
mod tests {
    use super::*;
    use mpd::Song;
    use std::collections::HashSet;
    use std::hash::Hasher;
    use std::hash::Hash;

    fn make_song(file: &str) -> Song {
        let mut song = Song::default();
        song.file = file.to_string();
        song
    }

    #[test]
    fn test_equality_same_file() {
        let song1 = HashableSong(make_song("music/foo.mp3"));
        let song2 = HashableSong(make_song("music/foo.mp3"));
        assert_eq!(song1, song2);
    }

    #[test]
    fn test_equality_different_file() {
        let song1 = HashableSong(make_song("music/foo.mp3"));
        let song2 = HashableSong(make_song("music/bar.mp3"));
        assert_ne!(song1, song2);
    }

    #[test]
    fn test_hashing_consistency() {
        use std::collections::hash_map::DefaultHasher;

        let song1 = HashableSong(make_song("music/foo.mp3"));
        let song2 = HashableSong(make_song("music/foo.mp3"));

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        song1.hash(&mut hasher1);
        song2.hash(&mut hasher2);

        assert_eq!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_hashing_differentiation() {
        use std::collections::hash_map::DefaultHasher;

        let song1 = HashableSong(make_song("music/foo.mp3"));
        let song2 = HashableSong(make_song("music/bar.mp3"));

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();

        song1.hash(&mut hasher1);
        song2.hash(&mut hasher2);

        assert_ne!(hasher1.finish(), hasher2.finish());
    }

    #[test]
    fn test_set_uniqueness() {
        let song1 = HashableSong(make_song("music/foo.mp3"));
        let song2 = HashableSong(make_song("music/foo.mp3"));
        let song3 = HashableSong(make_song("music/bar.mp3"));

        let mut set = HashSet::new();
        set.insert(song1);
        set.insert(song2); // should not be added again
        set.insert(song3);

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_from_impl() {
        let original = make_song("music/foo.mp3");
        let wrapped = HashableSong(original.clone());
        let unwrapped: Song = wrapped.into();
        assert_eq!(original.file, unwrapped.file);
    }
}
