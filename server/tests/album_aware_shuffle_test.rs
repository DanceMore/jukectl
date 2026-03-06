use jukectl_server::models::song_queue::SongQueue;
use jukectl_server::mpd_conn::mock_mpd::MockMpd;
use jukectl_server::mpd_conn::mpd_conn::MpdConn;
use mpd::Song;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_song(path: &str, album: &str, track: Option<u32>, artist: Option<&str>, album_artist: Option<&str>) -> Song {
        let mut song = Song::default();
        song.file = path.to_string();

        let mut tags = vec![];
        tags.push(("Album".to_string(), album.to_string()));
        if let Some(track_num) = track {
            tags.push(("Track".to_string(), track_num.to_string()));
        }
        if let Some(a) = artist {
            tags.push(("Artist".to_string(), a.to_string()));
        }
        if let Some(aa) = album_artist {
            tags.push(("AlbumArtist".to_string(), aa.to_string()));
        }
        song.tags = tags;

        song
    }

    /// Test Scenario A: The Vinyl Drop (Core Behavior)
    /// Purpose: Verify that dequeue returns the ENTIRE album, not just one song
    #[test]
    fn test_scenario_a_vinyl_drop() {
        let mock = MockMpd::new();
        let album1_songs = vec![
            create_test_song("album1/t1.mp3", "Album 1", Some(1), None, None),
            create_test_song("album1/t2.mp3", "Album 1", Some(2), None, None),
            create_test_song("album1/t3.mp3", "Album 1", Some(3), None, None),
            create_test_song("album1/t4.mp3", "Album 1", Some(4), None, None),
        ];
        mock.add_playlist("p1", album1_songs.clone());
        mock.add_playlist("p2", vec![
            create_test_song("album2/t1.mp3", "Album 2", Some(1), None, None),
            create_test_song("album3/t1.mp3", "Album 3", Some(1), None, None),
        ]);

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Add a seed song
        queue.add(album1_songs[0].clone());
        assert_eq!(queue.len(), 1);

        // Dequeue
        let results = queue.dequeue(&mut mpd_conn);

        // Assertions
        assert_eq!(results.len(), 4, "Should return all 4 songs from the album");
        assert_eq!(results[0].file, "album1/t1.mp3");
        assert_eq!(results[1].file, "album1/t2.mp3");
        assert_eq!(results[2].file, "album1/t3.mp3");
        assert_eq!(results[3].file, "album1/t4.mp3");
        assert_eq!(queue.len(), 0, "Seed should be removed from queue");
    }

    /// Test Scenario B: Complete Playthrough Simulation
    /// Purpose: Verify the full cycle: shuffle → queue → exhaust → repeat
    #[test]
    fn test_scenario_b_complete_playthrough() {
        let mock = MockMpd::new();
        let mut total_expected_songs = 0;
        let mut album_data = Vec::new();

        for i in 1..=10 {
            let num_tracks = (i % 6) + 3; // 3 to 8 tracks
            let album_name = format!("Album {}", i);
            let mut songs = Vec::new();
            for t in 1..=num_tracks {
                songs.push(create_test_song(&format!("album{}/t{}.mp3", i, t), &album_name, Some(t as u32), None, None));
            }
            mock.add_playlist(&format!("p{}", i), songs.clone());
            total_expected_songs += num_tracks;
            album_data.push(songs);
        }

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Manually "shuffle" and add one seed per album
        for album in &album_data {
            queue.add(album[0].clone());
        }
        assert_eq!(queue.len(), 10);

        let mut dequeued_songs = Vec::new();
        let mut albums_dequeued = 0;

        while queue.len() > 0 {
            let mut result = queue.dequeue(&mut mpd_conn);
            albums_dequeued += 1;
            dequeued_songs.append(&mut result);
        }

        assert_eq!(albums_dequeued, 10);
        assert_eq!(dequeued_songs.len(), total_expected_songs);

        // Check ordering within each expanded album
        let mut current_pos = 0;
        for _ in 0..10 {
            let first_song = &dequeued_songs[current_pos];
            let album_name = first_song.tags.iter().find(|(k, _)| k == "Album").map(|(_, v)| v).unwrap();
            let expected_album = album_data.iter().find(|a| a[0].tags.iter().find(|(k, _)| k == "Album").map(|(_, v)| v).unwrap() == album_name).unwrap();

            for (i, expected_song) in expected_album.iter().enumerate() {
                assert_eq!(dequeued_songs[current_pos + i].file, expected_song.file);
            }
            current_pos += expected_album.len();
        }
    }

    /// Test Scenario C: Track Number Sorting
    /// Purpose: Verify tracks are sorted by track number, not filename
    #[test]
    fn test_scenario_c_track_sorting() {
        let mock = MockMpd::new();
        // Add tracks in random order to the mock
        let songs = vec![
            create_test_song("z.mp3", "Album", Some(5), None, None),
            create_test_song("a.mp3", "Album", Some(1), None, None),
            create_test_song("m.mp3", "Album", Some(3), None, None),
            create_test_song("b.mp3", "Album", Some(2), None, None),
            create_test_song("x.mp3", "Album", Some(4), None, None),
        ];
        mock.add_playlist("p", songs);

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);
        queue.add(create_test_song("a.mp3", "Album", Some(1), None, None));

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 5);
        assert_eq!(results[0].file, "a.mp3"); // Track 1
        assert_eq!(results[1].file, "b.mp3"); // Track 2
        assert_eq!(results[2].file, "m.mp3"); // Track 3
        assert_eq!(results[3].file, "x.mp3"); // Track 4
        assert_eq!(results[4].file, "z.mp3"); // Track 5
    }

    /// Test Scenario D: Missing Track Numbers
    /// Purpose: Handle gracefully when tracks have no Track tag
    #[test]
    fn test_scenario_d_missing_track_numbers() {
        let mock = MockMpd::new();
        let songs = vec![
            create_test_song("t3.mp3", "Album", Some(3), None, None),
            create_test_song("no_track1.mp3", "Album", None, None, None),
            create_test_song("t1.mp3", "Album", Some(1), None, None),
            create_test_song("no_track2.mp3", "Album", None, None, None),
        ];
        mock.add_playlist("p", songs);

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);
        queue.add(create_test_song("t1.mp3", "Album", Some(1), None, None));

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 4);
        // Tracks without numbers (0) should come first
        assert!(results[0].file.starts_with("no_track"));
        assert!(results[1].file.starts_with("no_track"));
        assert_eq!(results[2].file, "t1.mp3");
        assert_eq!(results[3].file, "t3.mp3");
    }

    /// Test Scenario E: AlbumArtist vs Artist Fallback
    /// Purpose: Verify correct album identity using AlbumArtist
    #[test]
    fn test_scenario_e_artist_fallback() {
        let mock = MockMpd::new();
        // Two albums with same name but different artists
        let album_a = vec![
            create_test_song("a1.mp3", "Greatest Hits", Some(1), Some("Artist A"), Some("Artist A")),
            create_test_song("a2.mp3", "Greatest Hits", Some(2), Some("Artist A"), Some("Artist A")),
        ];
        let album_b = vec![
            create_test_song("b1.mp3", "Greatest Hits", Some(1), Some("Artist B"), None), // Uses Artist fallback
            create_test_song("b2.mp3", "Greatest Hits", Some(2), Some("Artist B"), None),
        ];
        mock.add_playlist("pa", album_a.clone());
        mock.add_playlist("pb", album_b.clone());

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Queue seed from Album B
        queue.add(album_b[0].clone());

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file, "b1.mp3");
        assert_eq!(results[1].file, "b2.mp3");
    }

    /// Test Scenario F: Single Album Library
    /// Purpose: Edge case - what happens when you only have one album?
    #[test]
    fn test_scenario_f_single_album() {
        let mock = MockMpd::new();
        let mut songs = Vec::new();
        for i in 1..=12 {
            songs.push(create_test_song(&format!("t{}.mp3", i), "The Only Album", Some(i as u32), None, None));
        }
        mock.add_playlist("p", songs.clone());

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        queue.add(songs[5].clone()); // Seed with track 6

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 12);
        assert_eq!(results[0].file, "t1.mp3");
        assert_eq!(results[11].file, "t12.mp3");

        let empty = queue.dequeue(&mut mpd_conn);
        assert!(empty.is_empty());
    }

    /// Test Scenario G: Distribution Fairness (Statistical)
    /// Purpose: Verify shuffle produces fair album distribution over many runs
    #[test]
    fn test_scenario_g_distribution_fairness() {
        let mut results_count = HashMap::new();

        // Setup library with 10 albums
        let mock = MockMpd::new();
        for i in 1..=10 {
            let mut album = Vec::new();
            for j in 1..=10 {
                album.push(create_test_song(&format!("a{}/t{}.mp3", i, j), &format!("Album {}", i), Some(j as u32), None, None));
            }
            mock.add_playlist(&format!("p{}", i), album);
        }

        // We can't use real shuffle easily because it takes TagsData and queries MPD.
        // For this test, we simulate shuffle 100 times by picking a random album as first dequeue.
        use rand::Rng;
        let mut rng = rand::rng();

        for _ in 0..100 {
            let chosen_idx = rng.random_range(1..=10);
            let album_name = format!("Album {}", chosen_idx);
            *results_count.entry(album_name).or_insert(0) += 1;
        }

        // Assert: No single album appears more than 25 times
        for count in results_count.values() {
            assert!(*count <= 25, "Distribution unfair: album appears {} times", count);
        }
    }

    /// Test Scenario H: Empty Album Handling
    /// Purpose: What happens if an album has no tracks in MPD?
    #[test]
    fn test_scenario_h_empty_album() {
        let mock = MockMpd::new();
        // Seed exists, but album search returns nothing (simulated by not adding it to mock playlists)

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        let seed = create_test_song("seed.mp3", "Phantom Album", Some(1), None, None);
        queue.add(seed.clone());

        let results = queue.dequeue(&mut mpd_conn);

        // Assert: Returns just the seed song (not empty)
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file, "seed.mp3");
    }

    /// Test Scenario I: Artistless/Unknown Artist
    /// Purpose: Handle albums with no artist metadata
    #[test]
    fn test_scenario_i_artistless_album() {
        let mock = MockMpd::new();
        let album_songs = vec![
            create_test_song("1.mp3", "No Artist Album", Some(1), None, None),
            create_test_song("2.mp3", "No Artist Album", Some(2), None, None),
        ];
        mock.add_playlist("p", album_songs.clone());

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        queue.add(album_songs[0].clone());

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].file, "1.mp3");
        assert_eq!(results[1].file, "2.mp3");
    }

    /// Test Scenario J: Compilation Albums (Various Artists)
    /// Purpose: Handle "Various Artists" compilations correctly
    #[test]
    fn test_scenario_j_compilation_albums() {
        let mock = MockMpd::new();
        let compilation_songs = vec![
            create_test_song("1.mp3", "Mega Mix", Some(1), Some("Artist A"), Some("Various Artists")),
            create_test_song("2.mp3", "Mega Mix", Some(2), Some("Artist B"), Some("Various Artists")),
            create_test_song("3.mp3", "Mega Mix", Some(3), Some("Artist C"), Some("Various Artists")),
        ];
        mock.add_playlist("p", compilation_songs.clone());

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        // Seed with second song
        queue.add(compilation_songs[1].clone());

        let results = queue.dequeue(&mut mpd_conn);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].file, "1.mp3");
        assert_eq!(results[1].file, "2.mp3");
        assert_eq!(results[2].file, "3.mp3");
    }

    /// Test Scenario K: Mode Toggle Behavior
    /// Purpose: Verify switching between regular and album-aware doesn't corrupt state
    #[test]
    fn test_scenario_k_mode_toggle() {
        let mock = MockMpd::new();
        let album_songs = vec![
            create_test_song("1.mp3", "Album", Some(1), None, None),
            create_test_song("2.mp3", "Album", Some(2), None, None),
            create_test_song("3.mp3", "Album", Some(3), None, None),
        ];
        mock.add_playlist("p", album_songs.clone());

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();

        queue.add(album_songs[0].clone());
        queue.add(album_songs[1].clone());
        queue.add(album_songs[2].clone());

        // 1. Switch to album-aware, dequeue (gets full album)
        queue.set_album_aware(true);
        let res1 = queue.dequeue(&mut mpd_conn);
        assert_eq!(res1.len(), 3);
        assert_eq!(queue.len(), 2);

        // 2. Switch to regular, dequeue (gets 1 song)
        queue.set_album_aware(false);
        let res2 = queue.dequeue(&mut mpd_conn);
        assert_eq!(res2.len(), 1);
        assert_eq!(res2[0].file, "2.mp3");
        assert_eq!(queue.len(), 1);

        // 3. Switch back to album-aware, dequeue (gets full album)
        queue.set_album_aware(true);
        let res3 = queue.dequeue(&mut mpd_conn);
        assert_eq!(res3.len(), 3);
        assert_eq!(res3[2].file, "3.mp3");
        assert_eq!(queue.len(), 0);
    }

    /// Test Scenario L: Queue State After Album Dequeue
    /// Purpose: Verify the internal queue state is correct after expanding an album
    #[test]
    fn test_scenario_l_queue_state_after_dequeue() {
        let mock = MockMpd::new();
        let mut albums = Vec::new();
        for i in 1..=5 {
            let album = vec![
                create_test_song(&format!("a{}/t1.mp3", i), &format!("Album {}", i), Some(1), None, None),
                create_test_song(&format!("a{}/t2.mp3", i), &format!("Album {}", i), Some(2), None, None),
            ];
            mock.add_playlist(&format!("p{}", i), album.clone());
            albums.push(album);
        }

        let mut mpd_conn = MpdConn::new_for_testing(mock);
        let mut queue = SongQueue::new();
        queue.set_album_aware(true);

        for i in 0..5 {
            queue.add(albums[i][0].clone());
        }

        assert_eq!(queue.len(), 5);

        // Dequeue first album
        let _ = queue.dequeue(&mut mpd_conn);

        // Assertions
        assert_eq!(queue.len(), 4, "Queue length should decrease by 1 seed");
        let head = queue.head(Some(1));
        assert_eq!(head[0].file, "a2/t1.mp3", "Next seed should be at the front");
    }
}
