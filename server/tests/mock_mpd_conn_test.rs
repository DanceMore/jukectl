use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use mpd::Song;

// Tests for the mock MPD implementation
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_song(path: &str) -> Song {
        let mut song = Song::default();
        song.file = path.to_string();
        song
    }

    #[test]
    fn test_playlist_operations() {
        let mock = MockMpd::new();

        // Add a playlist
        let songs = vec![
            create_test_song("test/song1.mp3"),
            create_test_song("test/song2.mp3"),
        ];
        mock.add_playlist("test_playlist", songs.clone());

        // Retrieve the playlist
        let result = mock.playlist("test_playlist").unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].file, "test/song1.mp3");

        // Add a song to the playlist
        let new_song = create_test_song("test/song3.mp3");
        mock.pl_push("test_playlist", new_song).unwrap();

        // Verify the song was added
        let updated = mock.playlist("test_playlist").unwrap();
        assert_eq!(updated.len(), 3);
        assert_eq!(updated[2].file, "test/song3.mp3");

        // Delete a song from the playlist
        mock.pl_delete("test_playlist", 1).unwrap();

        // Verify the song was removed
        let after_delete = mock.playlist("test_playlist").unwrap();
        assert_eq!(after_delete.len(), 2);
        assert_eq!(after_delete[0].file, "test/song1.mp3");
        assert_eq!(after_delete[1].file, "test/song3.mp3");
    }

    #[test]
    fn test_queue_operations() {
        let mock = MockMpd::new();

        // Initially queue should be empty
        let empty_queue = mock.queue().unwrap();
        assert_eq!(empty_queue.len(), 0);

        // Add songs to the queue
        let song1 = create_test_song("test/song1.mp3");
        let song2 = create_test_song("test/song2.mp3");

        mock.push(song1).unwrap();
        mock.push(song2).unwrap();

        // Verify queue state
        let queue = mock.queue().unwrap();
        assert_eq!(queue.len(), 2);
        assert_eq!(queue[0].file, "test/song1.mp3");

        // Delete a song
        mock.delete(0).unwrap();

        // Verify deletion
        let updated_queue = mock.queue().unwrap();
        assert_eq!(updated_queue.len(), 1);
        assert_eq!(updated_queue[0].file, "test/song2.mp3");
    }

    #[test]
    fn test_connection_simulation() {
        let mock = MockMpd::new();

        // Initially connected
        assert!(mock.ping().is_ok());

        // Simulate disconnect
        mock.simulate_disconnect();
        assert!(mock.ping().is_err());
        assert!(mock.queue().is_err());

        // Simulate reconnect
        mock.simulate_reconnect();
        assert!(mock.ping().is_ok());
        assert!(mock.queue().is_ok());
    }
}
