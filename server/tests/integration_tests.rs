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

    // Helper to wait for MPD to be ready
    fn wait_for_mpd_ready(max_attempts: u32) -> Result<(), Box<dyn std::error::Error>> {
        for attempt in 1..=max_attempts {
            println!("Attempting to connect to MPD (attempt {}/{})", attempt, max_attempts);
            
            match Client::connect("127.0.0.1:6600") {
                Ok(mut client) => {
                    match client.ping() {
                        Ok(_) => {
                            println!("MPD is ready!");
                            return Ok(());
                        }
                        Err(e) => println!("MPD ping failed: {}", e),
                    }
                }
                Err(e) => println!("MPD connection failed: {}", e),
            }
            
            if attempt < max_attempts {
                thread::sleep(Duration::from_secs(2));
            }
        }
        
        Err("MPD failed to become ready within timeout".into())
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
            eprintln!("⏭️  SKIPPED: test_real_mpd_connection (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        // Check if MPD is available (assumes it's already running)
        check_mpd_available()
            .expect("MPD container should be running before tests");

        // Test basic connection
        let mut mpd_conn = MpdConn::new()
            .expect("Failed to connect to test MPD");

        // Test ping
        mpd_conn.mpd.ping()
            .expect("Failed to ping MPD");

        println!("✓ MPD connection test passed");
    }

    #[test]
    fn test_real_album_search() {
        if !should_run_integration_tests() {
            eprintln!("⏭️  SKIPPED: test_real_album_search (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        check_mpd_available()
            .expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new()
            .expect("Failed to connect to test MPD");

        // Test album search functionality
        let mut query = Query::new();
        query.and(Term::Tag("album".into()), "Classic Rock Hits"); // Assuming this exists in test data
        
        let search_results = mpd_conn.mpd.search(&query, None)
            .expect("Failed to search for album");

        println!("Found {} songs in album", search_results.len());
        
        // Verify we got results
        assert!(!search_results.is_empty(), "Should find songs in test album");
        
        // Verify all songs are from the same album
        for song in &search_results {
            let album = song.tags.iter()
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

        check_mpd_available()
            .expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new()
            .expect("Failed to connect to test MPD");

        // Test playlist operations
        let test_playlist = "jukebox";
        
        // Clean up any existing playlist
        let _ = mpd_conn.mpd.pl_remove(test_playlist);

        // Search for a song to add to playlist
        let mut query = Query::new();
        query.and(Term::Tag("artist".into()), "The Test Rockers"); // Assuming this exists
        
        let songs = mpd_conn.mpd.search(&query, Some((0, 1)))
            .expect("Failed to search for test song");
        
        assert!(!songs.is_empty(), "Should find at least one test song");
        let test_song = &songs[0];

        // Create playlist and add song
        mpd_conn.mpd.pl_push(test_playlist, test_song.clone())
            .expect("Failed to add song to playlist");

        // Verify playlist contents
        let playlist_songs = mpd_conn.mpd.playlist(test_playlist)
            .expect("Failed to get playlist");
        
        assert_eq!(playlist_songs.len(), 1);
        assert_eq!(playlist_songs[0].file, test_song.file);

        // Clean up
        mpd_conn.mpd.pl_remove(test_playlist)
            .expect("Failed to remove test playlist");

        println!("✓ Playlist operations test passed");
    }

    #[test]
    fn test_real_album_aware_functionality() {
        if !should_run_integration_tests() {
            eprintln!("⏭️  SKIPPED: test_real_album_aware_functionality (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        check_mpd_available()
            .expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new()
            .expect("Failed to connect to test MPD");

        // Set up test data - create playlists with album representatives
        let rock_playlist = "test_rock";
        let jazz_playlist = "test_jazz";

        // Clean up any existing playlists
        let _ = mpd_conn.mpd.pl_remove(rock_playlist);
        let _ = mpd_conn.mpd.pl_remove(jazz_playlist);

        // Find some rock songs (assuming your test data has these)
        let mut rock_query = Query::new();
        rock_query.and(Term::Tag("genre".into()), "Rock");
        let rock_songs = mpd_conn.mpd.search(&rock_query, Some((0, 5)))
            .expect("Failed to find rock songs");

        // Find some jazz songs
        let mut jazz_query = Query::new();
        jazz_query.and(Term::Tag("genre".into()), "Jazz");
        let jazz_songs = mpd_conn.mpd.search(&jazz_query, Some((0, 5)))
            .expect("Failed to find jazz songs");

        // Add representative songs to playlists
        if !rock_songs.is_empty() {
            mpd_conn.mpd.pl_push(rock_playlist, rock_songs[0].clone())
                .expect("Failed to add rock song to playlist");
        }

        if !jazz_songs.is_empty() {
            mpd_conn.mpd.pl_push(jazz_playlist, jazz_songs[0].clone())
                .expect("Failed to add jazz song to playlist");
        }

        // Test TagsData with album-aware functionality
        let tags_data = TagsData {
            any: vec![],
            not: vec![],
            album_aware: true,
            album_tags: vec![rock_playlist.to_string(), jazz_playlist.to_string()],
        };

        let album_songs = tags_data.get_allowed_songs(&mut mpd_conn);
        
        // Should get full albums, not just representative songs
        println!("Found {} songs using album-aware mode", album_songs.len());
        
        // Test with SongQueue
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);
        queue.shuffle_and_add(album_songs);

        assert!(queue.len() > 0, "Queue should contain songs");

        // Clean up
        let _ = mpd_conn.mpd.pl_remove(rock_playlist);
        let _ = mpd_conn.mpd.pl_remove(jazz_playlist);

        println!("✓ Real album-aware functionality test passed");
    }

    #[test]
    fn test_stress_large_library() {
        if !should_run_integration_tests() {
            eprintln!("⏭️  SKIPPED: test_stress_large_library (set RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }

        check_mpd_available()
            .expect("MPD container should be running before tests");

        let mut mpd_conn = MpdConn::new()
            .expect("Failed to connect to test MPD");

        // Test performance with larger datasets
        let start_time = std::time::Instant::now();
        
        // Search for all songs
        let all_songs = mpd_conn.mpd.listall()
            .expect("Failed to list all songs");
        
        let list_time = start_time.elapsed();
        println!("Listed {} songs in {:?}", all_songs.len(), list_time);

        // Test search performance
        let search_start = std::time::Instant::now();
        let mut query = Query::new();
        query.and(Term::Any, "the"); // Common word
        
        let search_results = mpd_conn.mpd.search(&query, None)
            .expect("Failed to perform search");
        
        let search_time = search_start.elapsed();
        println!("Search found {} songs in {:?}", search_results.len(), search_time);

        // Performance assertions
        assert!(list_time.as_secs() < 10, "Library listing should complete within 10 seconds");
        assert!(search_time.as_secs() < 5, "Search should complete within 5 seconds");

        println!("✓ Stress test passed");
    }
}
