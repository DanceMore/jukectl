use jukectl_server::models::song_queue::SongQueue;

#[cfg(test)]
mod tests {
    use super::*;
    use mpd::Song;

    fn create_test_song(path: &str) -> Song {
        let mut song = Song::default();
        song.file = path.to_string();
        song
    }

    #[test]
    fn test_add_and_remove() {
        let mut queue = SongQueue::new();
        assert_eq!(queue.len(), 0);

        // Add a song
        let test_song = create_test_song("test/song1.mp3");
        queue.add(test_song.clone());
        assert_eq!(queue.len(), 1);

        // Remove the song
        let removed_song = queue.remove().unwrap();
        assert_eq!(removed_song.file, test_song.file);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_head_and_tail() {
        let mut queue = SongQueue::new();

        // Add several songs
        for i in 1..=5 {
            let song = create_test_song(&format!("test/song{}.mp3", i));
            queue.add(song);
        }

        // Test head with default count
        let head = queue.head(None);
        assert_eq!(head.len(), 3);
        assert_eq!(head[0].file, "test/song1.mp3");
        assert_eq!(head[2].file, "test/song3.mp3");

        // Test head with custom count
        let head = queue.head(Some(2));
        assert_eq!(head.len(), 2);

        // Test tail with default count
        let tail = queue.tail(None);
        assert_eq!(tail.len(), 3);
        assert_eq!(tail[0].file, "test/song3.mp3");
        assert_eq!(tail[2].file, "test/song5.mp3");

        // Test tail with custom count
        let tail = queue.tail(Some(2));
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].file, "test/song4.mp3");
        assert_eq!(tail[1].file, "test/song5.mp3");
    }

    #[test]
    fn test_empty_queue() {
        let mut queue = SongQueue::new();

        // Add several songs
        for i in 1..=3 {
            let song = create_test_song(&format!("test/song{}.mp3", i));
            queue.add(song);
        }

        assert_eq!(queue.len(), 3);

        // Empty the queue
        queue.empty_queue();
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_cache_invalidation() {
        let mut queue = SongQueue::new();
        
        // Test that cache can be invalidated without crashing
        queue.invalidate_cache();
        
        // Add some songs manually (not through shuffle)
        for i in 1..=5 {
            let song = create_test_song(&format!("test/song{}.mp3", i));
            queue.add(song);
        }
        
        assert_eq!(queue.len(), 5);
        
        println!("✓ Cache invalidation test passed");
    }

    #[test]
    fn test_cache_stats() {
        let queue = SongQueue::new();
        
        // Get initial cache stats
        let (hits, misses, hit_rate) = queue.cache_stats();
        
        // Initially should be 0/0
        assert_eq!(hits, 0);
        assert_eq!(misses, 0);
        assert_eq!(hit_rate, 0.0);
        
        println!("✓ Cache stats test passed");
    }

    #[test]
    fn test_manual_song_operations() {
        // Test that we can still manually add/remove songs
        // even though shuffle_and_add now needs MpdConn
        let mut queue = SongQueue::new();

        // Add songs manually
        for i in 1..=10 {
            let song = create_test_song(&format!("test/song{}.mp3", i));
            queue.add(song);
        }

        assert_eq!(queue.len(), 10);

        // Verify all songs are in the queue
        let all_songs = queue.head(Some(queue.len()));
        assert_eq!(all_songs.len(), 10);

        // Verify we can access them
        for i in 0..10 {
            assert_eq!(all_songs[i].file, format!("test/song{}.mp3", i + 1));
        }

        println!("✓ Manual song operations test passed");
    }
}
