use jukectl_server::models::hashable_song::HashableSong;
use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use mpd::Song;
use std::collections::HashSet;

#[cfg(test)]
mod tests {
    use super::*;

    // MockTagsData struct for testing album-aware functionality
    struct MockTagsData {
        album_tags: Vec<String>,
    }

    impl MockTagsData {
        fn get_songs_from_album(&self, mpd_client: &MockMpd, album_name: &str) -> Vec<Song> {
            let mut all_songs = vec![];

            if let Ok(rock_playlist) = mpd_client.playlist("rock") {
                for song in rock_playlist {
                    if get_tag_value(&song, "Album").as_deref() == Some(album_name) {
                        all_songs.push(song);
                    }
                }
            }

            if let Ok(jazz_playlist) = mpd_client.playlist("jazz") {
                for song in jazz_playlist {
                    if get_tag_value(&song, "Album").as_deref() == Some(album_name) {
                        all_songs.push(song);
                    }
                }
            }

            if let Ok(electronic_playlist) = mpd_client.playlist("electronic") {
                for song in electronic_playlist {
                    if get_tag_value(&song, "Album").as_deref() == Some(album_name) {
                        all_songs.push(song);
                    }
                }
            }

            all_songs
        }

        fn get_allowed_songs(&self, mpd_client: &MockMpd) -> HashSet<HashableSong> {
            let mut album_songs = HashSet::new();

            for tag in &self.album_tags {
                if let Ok(playlist) = mpd_client.playlist(tag) {
                    println!("[+] fetching album representatives from tag {}", tag);

                    for representative_song in playlist {
                        if let Some(album_name) = get_tag_value(&representative_song, "Album") {
                            println!("[+] expanding album: {}", album_name);

                            let album_songs_result = self.get_songs_from_album(mpd_client, &album_name);
                            for song in album_songs_result {
                                album_songs.insert(HashableSong(song));
                            }
                        }
                    }
                }
            }

            album_songs
        }
    }

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
    fn test_album_aware_shuffle() {
        // Create a mock MPD connection
        let mock_mpd = MockMpd::new();

        // Set up test albums in the mock MPD
        // Album 1: Classic Rock (3 tracks)
        let album1_songs = vec![
            create_test_song("album1/track1.mp3", "Classic Rock", Some(1)),
            create_test_song("album1/track2.mp3", "Classic Rock", Some(2)),
            create_test_song("album1/track3.mp3", "Classic Rock", Some(3)),
        ];

        // Album 2: Jazz Vibes (2 tracks)
        let album2_songs = vec![
            create_test_song("album2/track1.mp3", "Jazz Vibes", Some(1)),
            create_test_song("album2/track2.mp3", "Jazz Vibes", Some(2)),
        ];

        // Album 3: Electronic Beats (4 tracks)
        let album3_songs = vec![
            create_test_song("album3/track1.mp3", "Electronic Beats", Some(1)),
            create_test_song("album3/track2.mp3", "Electronic Beats", Some(2)),
            create_test_song("album3/track3.mp3", "Electronic Beats", Some(3)),
            create_test_song("album3/track4.mp3", "Electronic Beats", Some(4)),
        ];

        // Add albums to mock playlists
        mock_mpd.add_playlist("rock", album1_songs.clone());
        mock_mpd.add_playlist("jazz", album2_songs.clone());
        mock_mpd.add_playlist("electronic", album3_songs.clone());

        // Create a SongQueue with album-aware mode enabled
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Test with MockTagsData
        let tags_data = MockTagsData {
            album_tags: vec!["rock".to_string(), "jazz".to_string(), "electronic".to_string()],
        };

        // Get songs using the mock implementation
        let songs = tags_data.get_allowed_songs(&mock_mpd);

        // Add songs to queue (this would normally happen in shuffle_and_add)
        queue.shuffle_and_add(songs);

        // Verify results
        assert_eq!(queue.len(), 9); // 3 + 2 + 4 tracks

        // Get all songs from queue to verify order
        let all_songs = queue.head(Some(queue.len()));

        // Check that albums are complete and in proper track order
        // This is a bit tricky since the albums themselves are shuffled,
        // but we can verify internal album ordering

        // Helper function to extract album and track from song path
        fn get_album_and_track(song: &Song) -> (String, Option<u32>) {
            if let Some(album) = get_tag_value(song, "Album") {
                if let Some(track_str) = get_tag_value(song, "Track") {
                    return (album.clone(), track_str.parse().ok());
                }
            }
            (String::new(), None)
        }

        // Verify that each album's tracks are in order
        let mut current_album: Option<String> = None;
        let mut last_track: Option<u32> = None;

        for song in &all_songs {
            let (album, track) = get_album_and_track(song);

            if current_album.is_none() {
                current_album = Some(album.clone());
            }

            // If we're still on the same album, verify track order
            if current_album == Some(album.clone()) {
                if let (Some(current_track), Some(new_track)) = (last_track, track) {
                    assert!(new_track > current_track, "Tracks should be in order within albums");
                }
            } else {
                // Album changed - reset track tracking
                current_album = Some(album.clone());
            }

            last_track = track;
        }

        println!("Album-aware shuffle test passed!");
    }

    #[test]
    fn test_comparison_with_regular_shuffle() {
        // Create a mock MPD connection
        let mock_mpd = MockMpd::new();

        // Set up test albums in the mock MPD (same as above)
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

        // Create a simple set of songs (not grouped by album)
        let mut regular_songs = HashSet::new();
        for song in album1_songs.iter().chain(album2_songs.iter()) {
            regular_songs.insert(HashableSong(song.clone()));
        }

        // Move this outside the loop - this was the problem!
        regular_queue.shuffle_and_add(regular_songs);

        // Test album-aware shuffle
        let mut album_aware_queue = SongQueue::new();
        album_aware_queue.set_album_aware(true);

        // Use the same MockTagsData as above to get album-grouped songs
        let tags_data = MockTagsData {
            album_tags: vec!["rock".to_string(), "jazz".to_string()],
        };

        let album_songs = tags_data.get_allowed_songs(&mock_mpd);
        album_aware_queue.shuffle_and_add(album_songs);

        // Verify different behavior
        assert_eq!(regular_queue.len(), 5); // Individual songs
        assert_eq!(album_aware_queue.len(), 5); // Same number of songs

        // Get all songs from both queues
        let regular_all = regular_queue.head(Some(regular_queue.len()));
        let album_aware_all = album_aware_queue.head(Some(album_aware_queue.len()));

        // In regular mode, albums are mixed together
        // In album-aware mode, albums stay together

        // This is a basic check - in a real test we'd want to verify the actual ordering
        println!("Regular shuffle queue length: {}", regular_all.len());
        println!("Album-aware shuffle queue length: {}", album_aware_all.len());

        assert_ne!(regular_all, album_aware_all, "Queues should be different");
    }
}
