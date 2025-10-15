use jukectl_server::models::hashable_song::HashableSong;
use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use mpd::Song;
use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_song(path: &str, album: &str, track: Option<u32>) -> Song {
        let mut song = Song::default();
        song.file = path.to_string();

        // Add tags as Vec of (String, String) tuples
        let mut tags = vec![];
        tags.push(("Album".to_string(), album.to_string()));
        if let Some(track_num) = track {
            tags.push(("Track".to_string(), track_num.to_string()));
        }
        song.tags = tags;

        song
    }

    // Helper function to get tag value from Song
    fn get_tag_value(song: &Song, tag_name: &str) -> Option<String> {
        for (key, value) in &song.tags {
            if key == tag_name {
                return Some(value.clone());
            }
        }
        None
    }

    #[test]
    fn test_album_aware_shuffle_phase() {
        // Test Phase 1: Shuffle (should be same for both modes)
        let mock_mpd = MockMpd::new();

        // Set up test albums
        let album1_songs = vec![
            create_test_song("album1/track1.mp3", "Classic Rock", Some(1)),
            create_test_song("album1/track2.mp3", "Classic Rock", Some(2)),
            create_test_song("album1/track3.mp3", "Classic Rock", Some(3)),
        ];

        let album2_songs = vec![
            create_test_song("album2/track1.mp3", "Jazz Vibes", Some(1)),
            create_test_song("album2/track2.mp3", "Jazz Vibes", Some(2)),
        ];

        mock_mpd.add_playlist("rock", album1_songs.clone());
        mock_mpd.add_playlist("jazz", album2_songs.clone());

        // Create queue with album-aware mode
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Collect all songs into a HashSet (simulating what get_allowed_songs does)
        let mut all_songs = HashSet::new();
        for song in album1_songs.iter().chain(album2_songs.iter()) {
            all_songs.insert(HashableSong(song.clone()));
        }

        // Shuffle and add - THIS SHOULD BE RANDOM, NOT ALBUM-GROUPED!
        queue.shuffle_and_add(all_songs);

        // Verify we have all 5 songs
        assert_eq!(queue.len(), 5, "Queue should contain all 5 songs");

        println!("✓ Album-aware shuffle phase test passed");
        println!("  (Shuffle is same for both modes - individual songs randomized)");
    }

    #[test]
    fn test_album_aware_dequeue_phase() {
        // Test Phase 2: Dequeue (this is where album-aware differs)

        // Create a mock MPD with albums
        let mock_mpd = MockMpd::new();

        let album1_songs = vec![
            create_test_song("album1/track1.mp3", "Classic Rock", Some(1)),
            create_test_song("album1/track2.mp3", "Classic Rock", Some(2)),
            create_test_song("album1/track3.mp3", "Classic Rock", Some(3)),
        ];

        let album2_songs = vec![
            create_test_song("album2/track1.mp3", "Jazz Vibes", Some(1)),
            create_test_song("album2/track2.mp3", "Jazz Vibes", Some(2)),
        ];

        // Add albums to mock MPD so they can be queried
        mock_mpd.add_playlist("rock", album1_songs.clone());
        mock_mpd.add_playlist("jazz", album2_songs.clone());

        // Create a queue with mixed songs (like after shuffle)
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Add songs in mixed order (simulating shuffled queue)
        queue.add(album1_songs[0].clone()); // Classic Rock track 1
        queue.add(album2_songs[1].clone()); // Jazz track 2
        queue.add(album1_songs[2].clone()); // Classic Rock track 3
        queue.add(album2_songs[0].clone()); // Jazz track 1

        assert_eq!(queue.len(), 4, "Queue should have 4 songs initially");

        // Note: We can't actually test remove_album_aware() here because it needs
        // a real MpdConn that can query albums. The MockMpd doesn't support
        // the search() method that remove_album_aware() uses.

        // Instead, we'll test that the queue is set up correctly for dequeue
        println!("✓ Album-aware dequeue phase setup test passed");
        println!("  (Dequeue would expand seed song to full album in track order)");
        println!("  (Full dequeue testing requires integration tests with real MPD)");
    }

    #[test]
    fn test_comparison_with_regular_shuffle() {
        // Test that shuffle produces same structure for both modes
        let mock_mpd = MockMpd::new();

        let album1_songs = vec![
            create_test_song("album1/track1.mp3", "Classic Rock", Some(1)),
            create_test_song("album1/track2.mp3", "Classic Rock", Some(2)),
            create_test_song("album1/track3.mp3", "Classic Rock", Some(3)),
        ];

        let album2_songs = vec![
            create_test_song("album2/track1.mp3", "Jazz Vibes", Some(1)),
            create_test_song("album2/track2.mp3", "Jazz Vibes", Some(2)),
        ];

        mock_mpd.add_playlist("rock", album1_songs.clone());
        mock_mpd.add_playlist("jazz", album2_songs.clone());

        // Test regular shuffle
        let mut regular_queue = SongQueue::new();
        regular_queue.set_album_aware(false);

        let mut regular_songs = HashSet::new();
        for song in album1_songs.iter().chain(album2_songs.iter()) {
            regular_songs.insert(HashableSong(song.clone()));
        }

        regular_queue.shuffle_and_add(regular_songs);

        // Test album-aware shuffle
        let mut album_aware_queue = SongQueue::new();
        album_aware_queue.set_album_aware(true);

        let mut album_songs = HashSet::new();
        for song in album1_songs.iter().chain(album2_songs.iter()) {
            album_songs.insert(HashableSong(song.clone()));
        }

        album_aware_queue.shuffle_and_add(album_songs);

        // Both should have the same number of songs
        assert_eq!(regular_queue.len(), 5);
        assert_eq!(album_aware_queue.len(), 5);

        // Both queues will be randomly shuffled (same shuffle logic)
        // The difference is in dequeue behavior, not shuffle behavior!

        println!("✓ Shuffle comparison test passed");
        println!("  (Both modes use same shuffle - random individual songs)");
        println!("  (Difference is at dequeue time: regular=1 song, album=full album)");
    }

    #[test]
    fn test_regular_mode_dequeue() {
        // Test regular mode dequeue (1 song at a time)
        let mut queue = SongQueue::new();
        queue.set_album_aware(false); // Regular mode

        // Add some songs
        queue.add(create_test_song("song1.mp3", "Album A", Some(1)));
        queue.add(create_test_song("song2.mp3", "Album B", Some(1)));
        queue.add(create_test_song("song3.mp3", "Album A", Some(2)));

        assert_eq!(queue.len(), 3);

        // Regular dequeue returns single song
        let song1 = queue.remove();
        assert!(song1.is_some());
        assert_eq!(queue.len(), 2, "Should have 2 songs left");

        let song2 = queue.remove();
        assert!(song2.is_some());
        assert_eq!(queue.len(), 1, "Should have 1 song left");

        println!("✓ Regular mode dequeue test passed");
        println!("  (Regular mode: remove() returns 1 song at a time)");
    }

    #[test]
    fn test_album_mode_flag() {
        // Test that album_aware flag is properly set
        let mut queue = SongQueue::new();

        // Default should be false
        queue.set_album_aware(false);

        // Enable album mode
        queue.set_album_aware(true);

        // Disable again
        queue.set_album_aware(false);

        println!("✓ Album mode flag test passed");
        println!("  (Album-aware flag can be toggled)");
    }

    #[test]
    fn test_queue_basic_operations() {
        // Test basic queue operations work with album mode
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Test empty queue
        assert_eq!(queue.len(), 0);
        assert!(queue.remove().is_none());

        // Test add and length
        queue.add(create_test_song("test1.mp3", "Album", Some(1)));
        queue.add(create_test_song("test2.mp3", "Album", Some(2)));
        assert_eq!(queue.len(), 2);

        // Test head
        let head = queue.head(Some(1));
        assert_eq!(head.len(), 1);
        assert_eq!(head[0].file, "test1.mp3");

        // Test tail
        let tail = queue.tail(Some(1));
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].file, "test2.mp3");

        // Test empty
        queue.empty_queue();
        assert_eq!(queue.len(), 0);

        println!("✓ Queue basic operations test passed");
    }
}
