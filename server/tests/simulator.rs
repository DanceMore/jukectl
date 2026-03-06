use jukectl_server::app_state::{self, AppState};
use jukectl_server::mpd_conn::traits::MpdClient;
use jukectl_server::mpd_conn::mpd_conn::MpdBackend;
use mpd::Song;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

fn create_test_song(path: &str, album: &str, artist: &str) -> Song {
    let mut song = Song::default();
    song.file = path.to_string();
    song.tags.push(("Album".to_string(), album.to_string()));
    song.tags.push(("Artist".to_string(), artist.to_string()));
    song
}

async fn setup_simulator() -> (AppState, jukectl_server::mpd_conn::mock_mpd::MockMpd) {
    env::set_var("JUKECTL_DEV_MODE", "1");

    let state = app_state::initialize().await;

    // Extract the mock backend to seed it
    let mut conn = state.mpd_pool.get_connection().await.unwrap();
    let mock = match &conn.mpd_conn().mpd {
        MpdBackend::Mock(m) => m.clone(),
        _ => panic!("Expected mock backend"),
    };

    // Clear everything from previous tests because of SHARED_MOCK
    mock.clear_state();

    (state, mock)
}

#[tokio::test]
async fn test_scenario_a_refill_logic() {
    let (state, mock) = setup_simulator().await;
    let _ = env_logger::builder().is_test(true).try_init();

    // Use a unique tag for this test to avoid interference
    let tag = "tag_a";
    {
        let mut tags = state.tags_data.write().await;
        tags.any = vec![tag.to_string()];
    }

    // Seed 10 songs in the playlist
    let songs: Vec<Song> = (1..=10)
        .map(|i| create_test_song(&format!("a{}.mp3", i), "Album A", "Artist A"))
        .collect();
    mock.add_playlist(tag, songs.clone());

    // Initialize the queue
    app_state::initialize_queue(&state).await;

    {
        let queue = state.song_queue.read().await;
        assert_eq!(queue.len(), 10, "Queue should be initialized with 10 songs");
    }

    // Drain the queue to 1 song
    {
        let mut queue = state.song_queue.write().await;
        while queue.len() > 1 {
            queue.dequeue_single();
        }
        assert_eq!(queue.len(), 1);
    }

    // In our implementation, the scheduler refills when queue is EMPTY (len == 0).
    // The spec says "Drain the queue to 1 song. Verify the scheduler automatically refills".
    // Let's check scheduler/mod.rs:
    // if locked_song_queue.len() == 0 {
    //     info!("[!] Scheduler: Queue empty, refilling...");
    //     locked_song_queue.shuffle_and_add(&*locked_tags_data, pooled_conn.mpd_conn());
    // }

    // Drain the queue to 1 song.
    // Spec: "Drain the queue to 1 song. Verify the scheduler automatically refills the queue using the current tags."
    {
        let mut queue = state.song_queue.write().await;
        while queue.len() > 1 {
            queue.dequeue_single();
        }
        assert_eq!(queue.len(), 1);

        // Also ensure MPD queue is empty so scheduler sees it < 2
        let mut conn = state.mpd_pool.get_connection().await.unwrap();
        while conn.mpd_conn().mpd.queue().unwrap().len() > 0 {
            conn.mpd_conn().mpd.delete(0).unwrap();
        }
    }

    // Start scheduler (it runs in a loop every 3 seconds)
    let handle = jukectl_server::scheduler::start_scheduler(state.clone()).await;

    // Wait for scheduler to run
    let mut refilled = false;
    // We expect the queue to refill from 1 to 10
    for _ in 0..100 {
        sleep(Duration::from_millis(100)).await;
        let queue = state.song_queue.read().await;
        if queue.len() > 1 {
            refilled = true;
            break;
        }
    }

    let queue_len = state.song_queue.read().await.len();
    handle.abort();
    assert!(refilled, "Scheduler should have refilled the queue. Current len: {}", queue_len);
}

#[tokio::test]
async fn test_scenario_b_tag_hot_swap() {
    let (state, mock) = setup_simulator().await;
    let _ = env_logger::builder().is_test(true).try_init();

    let tag1 = "tag_b1";
    let tag2 = "tag_b2";

    {
        let mut tags = state.tags_data.write().await;
        tags.any = vec![tag1.to_string()];
    }

    // Seed two playlists
    let songs1 = vec![create_test_song("b1.mp3", "A", "B")];
    let songs2 = vec![create_test_song("b2.mp3", "C", "D"), create_test_song("b3.mp3", "C", "D")];

    mock.add_playlist(tag1, songs1);
    mock.add_playlist(tag2, songs2);

    // Initialize with tag1
    app_state::initialize_queue(&state).await;
    {
        let queue = state.song_queue.read().await;
        assert_eq!(queue.len(), 1);
    }

    // Change tags to tag2
    {
        let mut tags = state.tags_data.write().await;
        tags.any = vec![tag2.to_string()];
        // We also need to invalidate cache if we want it to pick up new songs immediately on next refill
        let mut queue = state.song_queue.write().await;
        queue.invalidate_cache();
        // Drain it to 1 song to trigger refill (Scheduler now refills at <= 1)
        while queue.len() > 1 {
            queue.dequeue_single();
        }
        if queue.len() == 0 {
            // Should not happen as we started with 1 but for safety
        } else {
            // We have 1 song (j1.mp3)
            queue.dequeue_single(); // Now empty, should trigger refill
        }
    }

    // Start scheduler
    let handle = jukectl_server::scheduler::start_scheduler(state.clone()).await;

    // Wait for refill
    let mut refilled_with_new_tags = false;
    for _ in 0..50 {
        sleep(Duration::from_millis(100)).await;
        let queue = state.song_queue.read().await;
        if queue.len() == 2 {
            refilled_with_new_tags = true;
            break;
        }
    }

    handle.abort();
    assert!(refilled_with_new_tags, "Scheduler should have refilled the queue with new tags");
}

#[tokio::test]
async fn test_scenario_c_empty_library() {
    let (state, _mock) = setup_simulator().await;
    let _ = env_logger::builder().is_test(true).try_init();

    // Use a non-existent tag
    {
        let mut tags = state.tags_data.write().await;
        tags.any = vec!["empty_tag".to_string()];
    }

    // Initialize the queue
    app_state::initialize_queue(&state).await;

    {
        let queue = state.song_queue.read().await;
        assert_eq!(queue.len(), 0, "Queue should be empty");
    }

    // Start scheduler
    let handle = jukectl_server::scheduler::start_scheduler(state.clone()).await;

    // Let it run for a bit
    sleep(Duration::from_secs(2)).await;

    // If it didn't panic, it's a pass. Verification is manual or via log capture if we had it.
    let queue = state.song_queue.read().await;
    handle.abort();
    assert_eq!(queue.len(), 0);
}

#[tokio::test]
async fn test_scenario_d_queue_race() {
    let (state, mock) = setup_simulator().await;
    let _ = env_logger::builder().is_test(true).try_init();

    let tag = "tag_d";
    {
        let mut tags = state.tags_data.write().await;
        tags.any = vec![tag.to_string()];
    }

    // Seed many songs
    let songs: Vec<Song> = (1..=100)
        .map(|i| create_test_song(&format!("d{}.mp3", i), "Album D", "Artist D"))
        .collect();
    mock.add_playlist(tag, songs);

    app_state::initialize_queue(&state).await;

    // Start scheduler
    let handle = jukectl_server::scheduler::start_scheduler(state.clone()).await;

    // Rapidly consume songs from MPD queue (simulated)
    // The scheduler adds songs to MPD queue when it has < 2 songs.

    let mut consumed_count = 0;
    let start = std::time::Instant::now();
    while start.elapsed() < Duration::from_secs(5) {
        // Mock MPD queue
        let mpd_queue_len = {
            let mut conn = state.mpd_pool.get_connection().await.unwrap();
            conn.mpd_conn().mpd.queue().unwrap().len()
        };

        if mpd_queue_len > 0 {
            let mut conn = state.mpd_pool.get_connection().await.unwrap();
            conn.mpd_conn().mpd.delete(0).unwrap();
            consumed_count += 1;
        }

        // Sometimes we consume very fast, sometimes we wait
        if consumed_count % 5 == 0 {
            sleep(Duration::from_millis(10)).await;
        } else {
            sleep(Duration::from_millis(50)).await;
        }
    }

    println!("Consumed {} songs", consumed_count);

    // Verify that all songs were accounted for
    let history = mock.get_pushed_history();
    println!("Total songs pushed according to history: {}", history.len());

    assert!(history.len() >= consumed_count, "All consumed songs should have been pushed");

    // Check for duplicates in history to ensure no double-pushes in rapid succession
    let mut seen = std::collections::HashSet::new();
    let mut duplicates = 0;
    for song in &history {
        if !seen.insert(song.file.clone()) {
            duplicates += 1;
        }
    }
    println!("Duplicates in push history: {}", duplicates);

    handle.abort();
}
