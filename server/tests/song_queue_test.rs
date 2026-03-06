#[cfg(test)]
mod tests {
    use jukectl_server::models::song_queue::SongQueue;
    use jukectl_server::mpd_conn::traits::Song;

    fn create_test_song(path: &str) -> Song {
        Song {
            file: path.to_string(),
            title: None,
            artist: None,
            album: None,
            duration: None,
            pos: None,
            id: None,
        }
    }

    #[test]
    fn test_song_queue_basic() {
        let mut queue = SongQueue::new();
        let song = create_test_song("test.mp3");
        queue.add(song);
        assert_eq!(queue.len(), 1);
        
        queue.empty_queue();
        assert_eq!(queue.len(), 0);
    }
}
