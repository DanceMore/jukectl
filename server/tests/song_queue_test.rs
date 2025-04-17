use jukectl_server::models::hashable_song::HashableSong;
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
    fn test_shuffle_and_add() {
        use std::collections::HashSet;

        let mut queue = SongQueue::new();
        let mut song_set = HashSet::new();

        // Create 10 songs as HashableSong objects
        for i in 1..=10 {
            let song = create_test_song(&format!("test/song{}.mp3", i));
            song_set.insert(HashableSong(song));
        }

        // Original set size should be 10
        assert_eq!(song_set.len(), 10);

        // After shuffling and adding, queue should have 10 items
        queue.shuffle_and_add(song_set);
        assert_eq!(queue.len(), 10);

        // Verify all songs are in the queue (just check count for now since order is randomized)
        let all_songs = queue.head(Some(queue.len()));
        assert_eq!(all_songs.len(), 10);

        // Create a set of song files to verify all original songs are present
        let mut result_files = HashSet::new();
        for song in all_songs {
            result_files.insert(song.file);
        }

        assert_eq!(result_files.len(), 10);
        for i in 1..=10 {
            assert!(result_files.contains(&format!("test/song{}.mp3", i)));
        }
    }
}
