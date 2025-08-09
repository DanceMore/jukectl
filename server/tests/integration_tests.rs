// tests/integration_tests.rs
use jukectl_server::models::hashable_song::HashableSong;
use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::models::tags_data::TagsData;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;
use mpd::{Client, Query, Term};
use std::collections::HashSet;
use std::env;
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Helper to check if we should run integration tests
    fn should_run_integration_tests() -> bool {
        env::var("RUN_INTEGRATION_TESTS").unwrap_or_default() == "1"
    }

    // Helper function to safely create and clean up test playlists
    fn manage_test_playlist(
        mpd_conn: &mut MpdConn,
        playlist_name: &str,
        song_to_add: Option<&mpd::Song>,
        should_create: bool,
        should_cleanup: bool
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // If we need to create the playlist and have a song to add
        if should_create && let Some(song) = song_to_add {
            // First try to remove any existing playlist (silently ignore errors)
            let _ = mpd_conn.mpd.pl_remove(playlist_name);
    
            // Create fresh playlist and add the song
            mpd_conn.mpd.pl_push(playlist_name, song.clone())?;
            println!("Created test playlist '{}' with song", playlist_name);
            Ok(true)
        } else if should_cleanup {
            // Clean up the playlist if requested
            let _ = mpd_conn.mpd.pl_remove(playlist_name);
            println!("Cleaned up test playlist '{}'", playlist_name);
            Ok(false)
        } else {
            // Just check if playlist exists
            match mpd_conn.mpd.playlist(playlist_name) {
                Ok(songs) => {
                    println!("Playlist '{}' exists with {} songs", playlist_name, songs.len());
                    Ok(!songs.is_empty())
                }
                Err(_) => {
                    println!("Playlist '{}' does not exist", playlist_name);
                    Ok(false)
                }
            }
        }
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

        // Check if MPD is available (assumes it's already running)
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
        query.and(Term::Tag("album".into()), "Classic Rock Hits"); // Assuming this exists in test data

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

        // Test playlist operations with safe helper
        let test_playlist = "test_playlist_ops"; // Use a dedicated test playlist instead of "jukebox"

        // Search for a song to add to playlist
        let mut query = Query::new();
        query.and(Term::Tag("artist".into()), "The Test Rockers"); // Assuming this exists

        let songs = mpd_conn
            .mpd
            .search(&query, Some((0, 1)))
            .expect("Failed to search for test song");

        assert!(!songs.is_empty(), "Should find at least one test song");
        let test_song = &songs[0];

        // Create playlist with our helper (creates if needed)
        manage_test_playlist(&mut mpd_conn, test_playlist, Some(test_song), true, false)
            .expect("Failed to create test playlist");

        // Verify playlist contents
        let playlist_songs = mpd_conn
            .mpd
            .playlist(test_playlist)
            .expect("Failed to get playlist");

        assert_eq!(playlist_songs.len(), 1);
        assert_eq!(playlist_songs[0].file, test_song.file);

        // Clean up using our helper
        manage_test_playlist(&mut mpd_conn, test_playlist, None, false, true)
            .expect("Failed to clean up test playlist");

        println!("✓ Playlist operations test passed");
    }

    #[test]
    fn test_real_album_aware_functionality() {
        if !should_run_integration_tests() {
            eprintln!("⏭️  SKIPPED: test_real_album_aware_functionality (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        check_mpd_available().expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new().expect("Failed to connect to test MPD");

        // Set up test data - create playlists with album representatives
        let rock_playlist = "test_rock";
        let jazz_playlist = "test_jazz";

        // Find some rock songs (assuming your test data has these)
        let mut rock_query = Query::new();
        rock_query.and(Term::Tag("genre".into()), "Rock");
        let rock_songs = mpd_conn
            .mpd
            .search(&rock_query, Some((0, 5)))
            .expect("Failed to find rock songs");

        // Find some jazz songs
        let mut jazz_query = Query::new();
        jazz_query.and(Term::Tag("genre".into()), "Jazz");
        let jazz_songs = mpd_conn
            .mpd
            .search(&jazz_query, Some((0, 5)))
            .expect("Failed to find jazz songs");

        // Add representative songs to playlists using our safe helper
        if !rock_songs.is_empty() {
            manage_test_playlist(
                &mut mpd_conn,
                rock_playlist,
                Some(&rock_songs[0]),
                true,
                false
            ).expect("Failed to create rock playlist");
        }

        if !jazz_songs.is_empty() {
            manage_test_playlist(
                &mut mpd_conn,
                jazz_playlist,
                Some(&jazz_songs[0]),
                true,
                false
            ).expect("Failed to create jazz playlist");
        }

        // Test TagsData with album-aware functionality
        let tags_data = TagsData {
            any: vec![],
            not: vec![],
        };

        let album_songs = tags_data.get_album_aware_songs(&mut mpd_conn);

        // Should get full albums, not just representative songs
        println!("Found {} songs using album-aware mode", album_songs.len());

        // Test with SongQueue
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);
        queue.shuffle_and_add(album_songs);

        assert!(queue.len() > 0, "Queue should contain songs");

        // Clean up using our safe helper
        if manage_test_playlist(&mut mpd_conn, rock_playlist, None, false, true).unwrap_or(false) {
            println!("Cleaned up rock playlist");
        }

        if manage_test_playlist(&mut mpd_conn, jazz_playlist, None, false, true).unwrap_or(false) {
            println!("Cleaned up jazz playlist");
        }

        println!("✓ Real album-aware functionality test passed");
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

        // Test search performance with common word
        let search_start = std::time::Instant::now();
        let mut query = Query::new();
        query.and(Term::Any, "the"); // Common word

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

        // Test with large album specifically
        let large_album_query_start = std::time::Instant::now();
        let mut large_album_query = Query::new();
        large_album_query.and(Term::Tag("album".into()), "100 Song Epic");

        let large_album_results = mpd_conn
            .mpd
            .search(&large_album_query, None)
            .expect("Failed to search for large album");

        let large_album_time = large_album_query_start.elapsed();
        println!(
            "Large album search found {} songs in {:?}",
            large_album_results.len(),
            large_album_time
        );

        // Test with edge case albums (Unicode, long names)
        let unicode_search_start = std::time::Instant::now();
        let mut unicode_query = Query::new();
        unicode_query.and(Term::Tag("album".into()), "世界音楽 Collection");

        let unicode_results = mpd_conn
            .mpd
            .search(&unicode_query, None)
            .expect("Failed to search for Unicode album");

        let unicode_time = unicode_search_start.elapsed();
        println!(
            "Unicode album search found {} songs in {:?}",
            unicode_results.len(),
            unicode_time
        );

        // Test with long names album
        let long_names_start = std::time::Instant::now();
        let mut long_names_query = Query::new();
        long_names_query.and(Term::Tag("album".into()), "This Album Title Is Also Ridiculously Long And Contains Many Words That Describe Nothing Important But Test String Length Handling");

        let long_names_results = mpd_conn
            .mpd
            .search(&long_names_query, None)
            .expect("Failed to search for long-named album");

        let long_names_time = long_names_start.elapsed();
        println!(
            "Long names album search found {} songs in {:?}",
            long_names_results.len(),
            long_names_time
        );

        // Performance assertions with stricter thresholds for large datasets
        assert!(
            list_time.as_secs() < 15,
            "Library listing should complete within 15 seconds"
        );
        assert!(
            search_time.as_secs() < 10,
            "Search should complete within 10 seconds"
        );
        assert!(
            large_album_time.as_secs() < 10,
            "Large album search should complete within 10 seconds"
        );
        assert!(
            unicode_time.as_secs() < 5,
            "Unicode album search should complete within 5 seconds"
        );
        assert!(
            long_names_time.as_secs() < 5,
            "Long names album search should complete within 5 seconds"
        );

        // Verify we found expected number of songs in large album
        assert!(
            large_album_results.len() >= 40, // Should have at least most of the 50 tracks
            "Should find majority of songs in large album"
        );

        println!("✓ Enhanced stress test passed");
    }
}
