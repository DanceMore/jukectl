// tests/integration_tests.rs
use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::models::tags_data::TagsData;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;
use mpd::{Client, Query, Term};
use std::env;
use std::thread;
use std::time::Duration;

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Helper to check if we should run integration tests
    fn should_run_integration_tests() -> bool {
        env::var("RUN_INTEGRATION_TESTS").unwrap_or_default() == "1"
    }

    // Helper to wait for MPD to be ready
    fn wait_for_mpd_ready(max_attempts: u32) -> Result<(), Box<dyn std::error::Error>> {
        for attempt in 1..=max_attempts {
            println!(
                "Attempting to connect to MPD (attempt {}/{})",
                attempt, max_attempts
            );

            match Client::connect("127.0.0.1:6600") {
                Ok(mut client) => match client.ping() {
                    Ok(_) => {
                        println!("MPD is ready!");
                        return Ok(());
                    }
                    Err(e) => println!("MPD ping failed: {}", e),
                },
                Err(e) => println!("MPD connection failed: {}", e),
            }

            if attempt < max_attempts {
                thread::sleep(Duration::from_secs(2));
            }
        }

        Err("MPD failed to become ready within timeout".into())
    }

    // Helper function to test "not" tags filtering
    fn test_not_tags_filtering(mpd_conn: &mut MpdConn) -> Result<(), Box<dyn std::error::Error>> {
        // First, search for all songs with "jukebox" tag
        let mut query = Query::new();
        query.and(Term::Tag("tag".into()), "jukebox");

        let all_jukebox_songs = mpd_conn.mpd.search(&query, None)?;
        println!("Found {} jukebox songs total", all_jukebox_songs.len());

        // Now search specifically for songs with both "jukebox" and "not-allowed" tags
        let mut not_query = Query::new();
        not_query.and(Term::Tag("tag".into()), "not-allowed");

        let not_allowed_songs = mpd_conn.mpd.search(&not_query, None)?;
        println!(
            "Found {} songs with 'not-allowed' tag",
            not_allowed_songs.len()
        );

        // Test tag filtering with exclusion logic
        let tags_data = TagsData {
            any: vec!["jukebox".to_string()],
            not: vec!["not-allowed".to_string()],
        };

        let filtered_songs = tags_data.get_allowed_songs(mpd_conn);

        // Should only include jukebox songs without "not-allowed" tag
        assert!(filtered_songs.len() < all_jukebox_songs.len());
        assert_eq!(
            filtered_songs.len(),
            all_jukebox_songs.len() - not_allowed_songs.len()
        );

        Ok(())
    }

    // Helper function to test Unicode handling
    fn test_unicode_handling(mpd_conn: &mut MpdConn) -> Result<(), Box<dyn std::error::Error>> {
        // Search for the Unicode album
        let mut query = Query::new();
        query.and(Term::Tag("album".into()), "世界音楽 Collection");

        let search_results = mpd_conn.mpd.search(&query, None)?;
        println!("Found {} songs in Unicode album", search_results.len());

        assert!(
            !search_results.is_empty(),
            "Should find songs in Unicode album"
        );

        // Verify all songs have correct album name (Unicode)
        for song in &search_results {
            let album = song
                .tags
                .iter()
                .find(|(key, _)| key == "Album")
                .map(|(_, value)| value);
            assert_eq!(album, Some(&"世界音楽 Collection".to_string()));
        }

        Ok(())
    }

    // Helper function to test long names handling
    fn test_long_names_handling(mpd_conn: &mut MpdConn) -> Result<(), Box<dyn std::error::Error>> {
        // Search for the album with long names
        let mut query = Query::new();
        query.and(Term::Tag("album".into()), "This Album Title Is Also Ridiculously Long And Contains Many Words That Describe Nothing Important But Test String Length Handling");

        let search_results = mpd_conn.mpd.search(&query, None)?;
        println!("Found {} songs in long-named album", search_results.len());

        assert!(
            !search_results.is_empty(),
            "Should find songs in long-named album"
        );

        // Verify all songs have the correct (long) album name
        for song in &search_results {
            let album = song
                .tags
                .iter()
                .find(|(key, _)| key == "Album")
                .map(|(_, value)| value);
            assert_eq!(album, Some(&"This Album Title Is Also Ridiculously Long And Contains Many Words That Describe Nothing Important But Test String Length Handling".to_string()));
        }

        Ok(())
    }

    // Helper function to test minimal metadata handling
    fn test_minimal_metadata_handling(
        mpd_conn: &mut MpdConn,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Search for the album with minimal data
        let mut query = Query::new();
        query.and(Term::Tag("artist".into()), "Unknown");

        let search_results = mpd_conn.mpd.search(&query, None)?;
        println!(
            "Found {} songs in minimal metadata album",
            search_results.len()
        );

        assert!(
            !search_results.is_empty(),
            "Should find songs in minimal metadata album"
        );

        // Verify the songs have minimal data but are still functional
        for song in &search_results {
            let artist = song
                .tags
                .iter()
                .find(|(key, _)| key == "Artist")
                .map(|(_, value)| value);
            assert_eq!(artist, Some(&"Unknown".to_string()));

            // Album should be empty string but file path should exist
            assert!(song.file.contains("/"), "Song file path should be valid");
        }

        Ok(())
    }

    // Helper to check if MPD is available (assumes it's already running)
    fn check_mpd_available() -> Result<(), Box<dyn std::error::Error>> {
        match wait_for_mpd_ready(3) {
            Ok(_) => Ok(()),
            Err(e) => {
                eprintln!("❌ MPD not available. Please start it first with:");
                eprintln!("   docker-compose -f docker-compose.test.yml up -d");
                eprintln!("   or: make test-setup");
                Err(e)
            }
        }
    }

    #[test]
    fn test_real_mpd_connection() {
        if !should_run_integration_tests() {
            eprintln!(
                "⏭️  SKIPPED: test_real_mpd_connection (set RUN_INTEGRATION_TESTS=1 to enable)"
            );
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        // Test basic connection
        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Test ping
        mpd_conn.mpd.ping().expect("Failed to ping MPD");

        println!("✓ MPD connection test passed");
    }

    #[test]
    fn test_real_album_search() {
        if !should_run_integration_tests() {
            eprintln!(
                "⏭️  SKIPPED: test_real_album_search (set RUN_INTEGRATION_TESTS=1 to enable)"
            );
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Test album search functionality
        let mut query = Query::new();
        query.and(Term::Tag("album".into()), "Classic Rock Hits");

        let search_results = mpd_conn
            .mpd
            .search(&query, None)
            .expect("Failed to search for album");

        println!("Found {} songs in album", search_results.len());

        // Verify we got results
        assert!(
            !search_results.is_empty(),
            "Should find songs in test album"
        );

        // Verify all songs are from the same album
        for song in &search_results {
            let album = song
                .tags
                .iter()
                .find(|(key, _)| key == "Album")
                .map(|(_, value)| value);
            assert_eq!(album, Some(&"Classic Rock Hits".to_string()));
        }

        println!("✓ Album search test passed");
    }

    #[test]
    fn test_real_playlist_operations() {
        if !should_run_integration_tests() {
            eprintln!("⏭️  SKIPPED: test_real_playlist_operations (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Test playlist operations
        let test_playlist = "jukebox";

        // Clean up any existing playlist
        let _ = mpd_conn.mpd.pl_remove(test_playlist);

        // Search for a song to add to playlist
        let mut query = Query::new();
        query.and(Term::Tag("artist".into()), "The Test Rockers");

        let songs = mpd_conn
            .mpd
            .search(&query, Some((0, 1)))
            .expect("Failed to search for test song");

        assert!(!songs.is_empty(), "Should find at least one test song");
        let test_song = &songs[0];

        // Create playlist and add song
        mpd_conn
            .mpd
            .pl_push(test_playlist, test_song.clone())
            .expect("Failed to add song to playlist");

        // Verify playlist contents
        let playlist_songs = mpd_conn
            .mpd
            .playlist(test_playlist)
            .expect("Failed to get playlist");

        assert_eq!(playlist_songs.len(), 1);
        assert_eq!(playlist_songs[0].file, test_song.file);

        // Clean up
        mpd_conn
            .mpd
            .pl_remove(test_playlist)
            .expect("Failed to remove test playlist");

        println!("✓ Playlist operations test passed");
    }

    #[test]
    fn test_real_shuffle_with_tags() {
        if !should_run_integration_tests() {
            eprintln!(
                "⏭️  SKIPPED: test_real_shuffle_with_tags (set RUN_INTEGRATION_TESTS=1 to enable)"
            );
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Create tags data
        let tags_data = TagsData {
            any: vec!["jukebox".to_string()],
            not: vec!["explicit".to_string()],
        };

        // Get allowed songs using the new API
        let allowed_songs = tags_data.get_allowed_songs(&mut mpd_conn);
        println!("Found {} allowed songs", allowed_songs.len());

        // Test with SongQueue
        let mut queue = SongQueue::new();
        queue.set_album_aware(false);

        // Use the new shuffle_and_add API
        queue.shuffle_and_add(&tags_data, &mut mpd_conn);

        assert!(queue.len() > 0, "Queue should contain songs");

        println!("✓ Real shuffle with tags test passed");
    }

    #[test]
    fn test_stress_large_library() {
        if !should_run_integration_tests() {
            eprintln!(
                "⏭️  SKIPPED: test_stress_large_library (set RUN_INTEGRATION_TESTS=1 to enable)"
            );
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Test performance with larger datasets
        let start_time = std::time::Instant::now();

        // Search for all songs
        let all_songs = mpd_conn.mpd.listall().expect("Failed to list all songs");

        let list_time = start_time.elapsed();
        println!("Listed {} songs in {:?}", all_songs.len(), list_time);

        // Test search performance
        let search_start = std::time::Instant::now();
        let mut query = Query::new();
        query.and(Term::Any, "the");

        let search_results = mpd_conn
            .mpd
            .search(&query, None)
            .expect("Failed to perform search");

        let search_time = search_start.elapsed();
        println!(
            "Search found {} songs in {:?}",
            search_results.len(),
            search_time
        );

        // Performance assertions
        assert!(
            list_time.as_secs() < 10,
            "Library listing should complete within 10 seconds"
        );
        assert!(
            search_time.as_secs() < 5,
            "Search should complete within 5 seconds"
        );

        println!("✓ Stress test passed");
    }
}
