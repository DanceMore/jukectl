#[cfg(test)]
mod tests {
    use jukectl_server::models::hashable_song::HashableSong;
    use jukectl_server::mpd_conn::traits::Song;
    use std::collections::HashSet;

    #[test]
    fn test_hashable_song_equality() {
        let song1 = Song {
            file: "test.mp3".to_string(),
            title: Some("Title".to_string()),
            artist: Some("Artist".to_string()),
            album: None,
            duration: None,
            pos: None,
            id: None,
        };
        let song2 = Song {
            file: "test.mp3".to_string(),
            title: Some("Other Title".to_string()),
            artist: Some("Other Artist".to_string()),
            album: None,
            duration: None,
            pos: None,
            id: None,
        };

        let hs1 = HashableSong(song1);
        let hs2 = HashableSong(song2);

        assert_eq!(hs1, hs2);
    }

    #[test]
    fn test_hashable_song_in_set() {
        let mut set = HashSet::new();
        let song1 = Song {
            file: "test.mp3".to_string(),
            title: None,
            artist: None,
            album: None,
            duration: None,
            pos: None,
            id: None,
        };
        
        set.insert(HashableSong(song1.clone()));
        assert!(set.contains(&HashableSong(song1)));
    }
}
