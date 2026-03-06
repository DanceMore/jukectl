---
title: Behavioral Simulator (Scheduler Testing)
lifecycle: building
status: [CURRENT]
author: neoice
created: 2026-03-05
depends_on:
  - 0001-vinyl-jukebox
---

# SPEC 0003: Behavioral Simulator

## Overview

This spec defines the behavioral simulator: a test infrastructure for verifying the scheduler's runtime behavior without requiring a real MPD server.

**The Hardware Wall**: Jules cannot access your local MPD server. The behavioral simulator lets us test the scheduler loop in a deterministic, repeatable way.

---

## What the Scheduler Does

The scheduler (`server/src/scheduler/mod.rs`) runs a tick every 3 seconds:

```
FOR EACH TICK:
1. Get MPD connection from pool
2. Lock song_queue and tags_data
3. IF internal queue is empty (len == 0):
   - Shuffle and refill from current tags
4. Get MPD queue length
5. IF MPD queue < 2 songs:
   - Call dequeue() to get more songs
   - Push songs to MPD queue
   - Call play() if needed
6. Release locks
7. Sleep 3 seconds
```

**CRITICAL**: Refill happens at `len == 0`, NOT `len == 1`.

---

## Behavioral Test Scenarios

### Scenario A: Refill Logic

**What we're testing**: Does the scheduler refill the queue when it becomes empty?

**Setup**:
1. Seed mock with 10 songs in playlist "tag_a"
2. Initialize queue (loads 10 songs)
3. Drain internal queue to 0 songs

**Expected behavior**:
- Scheduler detects queue is empty (len == 0)
- Scheduler calls `shuffle_and_add()` with current tags
- Queue now has 10 new songs

**Test assertion**: `queue.len() == 10` after scheduler tick

### Scenario B: Tag Hot-Swap

**What we're testing**: Does the scheduler pick up tag changes?

**Setup**:
1. Seed mock with playlist "tag_a" (5 songs)
2. Seed mock with playlist "tag_b" (5 songs)  
3. Initialize queue with tag_a
4. Let scheduler run a few ticks

**Action**:
- Change tags from "tag_a" to "tag_b"
- Invalidate cache

**Expected behavior**:
- Scheduler detects tags changed (cache invalidated)
- On next refill, uses NEW tags (tag_b)
- Queue now has tag_b songs

**Test assertion**: After tag change + refill, queue contains tag_b songs

### Scenario C: Empty Library

**What we're testing**: Does the scheduler handle gracefully when tags point to empty playlist?

**Setup**:
1. Set tags to "nonexistent_playlist"
2. Initialize queue

**Expected behavior**:
- Scheduler logs warning
- No panic
- Queue remains empty

**Test assertion**: No panic, warning logged

### Scenario D: Queue Race

**What we're testing**: Does the scheduler handle rapid consumption correctly?

**Setup**:
1. Seed queue with 10 songs
2. Start scheduler

**Action**:
- Rapidly drain MPD queue (simulate fast playback)
- While scheduler is running

**Expected behavior**:
- Scheduler refills before queue runs dry
- No songs lost
- No duplicates

**Test assertion**: All 10 songs eventually played, no duplicates

---

## Test Infrastructure Requirements

### Isolated Mocks

**IMPORTANT**: Each test must have its own MockMpd instance. Do NOT use a shared static mockCOR.

**RECT**:
```rust
// Each test gets fresh mock
let mock = MockMpd::new();
let mpd_conn = MpdConn::new_for_testing(mock);
```

**WRONG** (causes test pollution):
```rust
// Shared across tests - BAD
static SHARED_MOCK: OnceLock<MockMpd> = OnceLock::new();
```

### MockMpd Requirements

For scheduler testing, MockMpd must:

1. **track pushed songs** - `pushed_history` to verify what was added
2. **clear state** - `clear_state()` between tests
3. **support queue()** - Return current queue
4. **support push()** - Add to queue and track history
5. **support delete()** - Remove from queue
6. **support playlist()** - Return songs from a playlist

### Using MpdConn::new_for_testing()

```rust
// In server/src/mpd_conn/mpd_conn.rs
impl MpdConn {
    /// Create a new MpdConn for testing with a specific MockMpd
    pub fn new_for_testing(mock: MockMpd) -> Self {
        MpdConn {
            mpd: MpdBackend::Mock(mock),
            address: Vec::new(),
            host: "mock".to_string(),
            port: 0,
            is_dev_mode: true,
        }
    }
}
```

---

## Canonical Test Fixtures

For scheduler/behavioral tests, we need a simulation universe - a controlled environment with predictable state transitions.

### The Scheduler Simulation Fixture

Create `server/tests/fixtures/scheduler_simulation.rs`:

```rust
use mpd::Song;
use std::collections::VecDeque;

/// Represents a snapshot of scheduler state at a point in time.
/// Used to verify behavior across ticks.
#[derive(Debug, Clone)]
pub struct SchedulerSnapshot {
    pub tick: u64,
    pub internal_queue_len: usize,
    pub mpd_queue_len: usize,
    pub current_tags: Vec<String>,
    pub songs_added_this_tick: Vec<String>,  // filenames
    pub songs_played_this_tick: Vec<String>, // filenames
}

/// A timeline of scheduler events - the "simulation"
#[derive(Debug, Clone)]
pub struct SchedulerTimeline {
    pub snapshots: Vec<SchedulerSnapshot>,
    pub events: Vec<SchedulerEvent>,
}

#[derive(Debug, Clone)]
pub enum SchedulerEvent {
    Tick(u64),
    TagsChanged { from: Vec<String>, to: Vec<String> },
    QueueDrained { count: usize },
    Refill { song_count: usize },
    CacheInvalidated,
}

/// Builder for creating test scenarios
pub struct ScenarioBuilder {
    initial_songs: Vec<Song>,
    initial_tags: Vec<String>,
    timeline_events: Vec<SchedulerEvent>,
}

impl ScenarioBuilder {
    /// Create a fresh scenario with library and tags
    pub fn new() -> Self {
        ScenarioBuilder {
            initial_songs: Vec::new(),
            initial_tags: vec!["jukebox".to_string()],
            timeline_events: Vec::new(),
        }
    }

    /// Add songs to the "library" (mock playlists)
    pub fn with_library(mut self, songs: Vec<Song>) -> Self {
        self.initial_songs = songs;
        self
    }

    /// Set initial tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.initial_tags = tags;
        self
    }

    /// Schedule a tag change at some point
    pub fn tag_change_at(self, tick: u64, new_tags: Vec<String>) -> Self {
        let mut events = self.timeline_events;
        events.push(SchedulerEvent::TagsChanged { 
            from: self.initial_tags.clone(), 
            to: new_tags.clone() 
        });
        ScenarioBuilder {
            initial_songs: self.initial_songs,
            initial_tags: self.initial_tags,
            timeline_events: events,
        }
    }

    /// Schedule queue drain at some point
    pub fn drain_at(self, tick: u64, count: usize) -> Self {
        let mut events = self.timeline_events;
        events.push(SchedulerEvent::QueueDrained { count });
        ScenarioBuilder {
            initial_songs: self.initial_songs,
            initial_tags: self.initial_tags,
            timeline_events: events,
        }
    }

    pub fn build(self) -> TestScenario {
        TestScenario {
            initial_songs: self.initial_songs,
            initial_tags: self.initial_tags,
            timeline: SchedulerTimeline {
                snapshots: Vec::new(),
                events: self.timeline_events,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestScenario {
    pub initial_songs: Vec<Song>,
    pub initial_tags: Vec<String>,
    pub timeline: SchedulerTimeline,
}

/// COMMON SCENARIOS (Pre-built fixtures)

/// Scenario 1: Basic refill on empty queue
pub fn scenario_refill_on_empty() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .build()
}

/// Scenario 2: Tag hot-swap mid-playback
pub fn scenario_tag_hot_swap() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .tag_change_at(5, vec!["jazz".to_string()])  // Change at tick 5
        .build()
}

/// Scenario 3: Rapid drain (user skipping)
pub fn scenario_rapid_drain() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["music".to_string()])
        .with_library(sample_rock_library())
        .drain_at(3, 5)   // Drain 5 songs at tick 3
        .drain_at(4, 3)   // Drain 3 more at tick 4
        .build()
}

/// Scenario 4: Empty library
pub fn scenario_empty_library() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["empty_tag".to_string()])
        .with_library(Vec::new())  // No songs!
        .build()
}

/// Scenario 5: Cache behavior
pub fn scenario_cache_invalidation() -> TestScenario {
    ScenarioBuilder::new()
        .with_tags(vec!["rock".to_string()])
        .with_library(sample_rock_library())
        .tag_change_at(3, vec!["pop".to_string()])  // Invalidate cache
        .tag_change_at(6, vec!["rock".to_string()]) // Back to rock
        .build()
}

// ============================================
// HELPER: Sample libraries for scenarios
// ============================================

fn sample_rock_library() -> Vec<Song> {
    vec![
        mk_song("rock/artist1/album1/track1.mp3", "Artist1", "Album1", 1),
        mk_song("rock/artist1/album1/track2.mp3", "Artist1", "Album1", 2),
        mk_song("rock/artist2/album2/track1.mp3", "Artist2", "Album2", 1),
        mk_song("rock/artist2/album2/track2.mp3", "Artist2", "Album2", 2),
        mk_song("rock/artist3/album3/track1.mp3", "Artist3", "Album3", 1),
        // ... more songs
    ]
}

fn mk_song(path: &str, artist: &str, album: &str, track: u32) -> Song {
    let mut song = Song::default();
    song.file = path.to_string();
    song.tags = vec![
        ("Artist".to_string(), artist.to_string()),
        ("Album".to_string(), album.to_string()),
        ("Track".to_string(), track.to_string()),
    ];
    song
}
```

### Using the Fixtures in Tests

```rust
#[tokio::test]
async fn test_refill_on_empty() {
    // Load the pre-built scenario
    let scenario = scenario_refill_on_empty();
    
    // Initialize with scenario data
    let mock = MockMpd::new();
    mock.add_playlist("rock", scenario.initial_songs.clone());
    
    let state = create_test_state(mock, scenario.initial_tags).await;
    
    // Run scheduler for a few ticks
    run_scheduler_ticks(&state, 5).await;
    
    // Verify: queue should be refilled
    let queue = state.song_queue.read().await;
    assert!(queue.len() > 0);
}

#[tokio::test]
async fn test_tag_hot_swap() {
    let scenario = scenario_tag_hot_swap();
    
    // ... setup ...
    
    // Run to tick 4 (before tag change)
    run_scheduler_ticks(&state, 4).await;
    
    // Verify: using rock songs
    let queue = state.song_queue.read().await;
    let tags = state.tags_data.read().await;
    // Should have rock songs...
    
    // Now change tags (simulating what scenario defines)
    change_tags(&state, vec!["jazz".to_string()]).await;
    
    // Run more ticks
    run_scheduler_ticks(&state, 3).await;
    
    // Verify: now using JAZZ songs
    // ...
}
```

---

## Common Mistakes

### ❌ Wrong: Testing implementation details

```rust
// Testing internal cache state - NOT a behavioral test
assert!(queue.cache.is_valid());
```

### ✅ Correct: Testing observable behavior

```rust
// Testing what happens when queue empties - BEHAVIORAL test
let initial_len = queue.len();
drain_queue_to_empty(&mut queue);
scheduler.tick().await;
assert!(queue.len() > initial_len);  // Refilled!
```

### ❌ Wrong: Expecting refill at len == 1

```rust
// Scheduler refills at len == 0, not len == 1
while queue.len() > 1 {  // Wrong!
    queue.dequeue_single();
}
// Scheduler will NOT refill here!
```

### ✅ Correct: Drain to zero for refill

```rust
// Scheduler refills when len == 0
while queue.len() > 0 {
    queue.dequeue_single();
}
// NOW scheduler will refill
```

---

## Test File Location

- **File**: `server/tests/simulator.rs`
- **Run**: `cargo test --test simulator`

---

## Related Specs

- [SPEC 0001: Vinyl Jukebox Philosophy](./0001-vinyl-jukebox.md)
- [SPEC 0002: Album-Aware Shuffle](./0002-album-aware-shuffle.md)
- [SPEC 0004: Mock Infrastructure](./0004-mock-infrastructure.md)

---

## Changelog

- 2026-03-05: Initial spec - neoice
