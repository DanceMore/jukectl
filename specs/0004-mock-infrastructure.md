---
title: Mock Infrastructure
lifecycle: building
status: [CURRENT]
author: neoice
created: 2026-03-05
depends_on:
  - 0001-vinyl-jukebox
---

# SPEC 0004: Mock Infrastructure

## Overview

This spec defines the MockMpd infrastructure—the in-memory MPD substitute that allows testing without a real MPD server.

**The Hardware Wall**: Jules cannot access your local MPD server. MockMpd bypasses this by implementing the same interface as a real MPD connection.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        App Code                             │
│  (SongQueue, TagsData, Scheduler)                          │
└─────────────────────────┬───────────────────────────────────┘
                          │ uses MpdClient trait
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                      MpdBackend                             │
│  ┌─────────────────┐    ┌────────────────────────────────┐ │
│  │ Real(Client)    │    │ Mock(MockMpd)                  │ │
│  │ (real MPD)     │    │ (in-memory, test only)         │ │
│  └─────────────────┘    └────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

---

## Files

- `server/src/mpd_conn/traits.rs` - `MpdClient` trait definition
- `server/src/mpd_conn/mock_mpd.rs` - Mock implementation
- `server/src/mpd_conn/mpd_conn.rs` - `MpdBackend` enum + `MpdConn`

---

## MpdClient Trait

```rust
pub trait MpdClient: Send + Sync {
    fn ping(&mut self) -> Result<()>;
    fn playlist(&mut self, name: &str) -> Result<Vec<Song>>;
    fn playlists(&mut self) -> Result<Vec<Playlist>>;
    fn queue(&mut self) -> Result<Vec<Song>>;
    fn search(&mut self, query: &Query, window: Option<(u32, u32)>) -> Result<Vec<Song>>;
    fn consume(&mut self, state: bool) -> Result<()>;
    fn push(&mut self, song: Song) -> Result<mpd::Id>;
    fn delete(&mut self, pos: u32) -> Result<()>;
    fn play(&mut self) -> Result<()>;
    fn pl_push(&mut self, playlist: &str, song: Song) -> Result<()>;
    fn pl_delete(&mut self, playlist: &str, pos: u32) -> Result<()>;
    fn pl_remove(&mut self, playlist: &str) -> Result<()>;
    fn listall(&mut self) -> Result<Vec<Song>>;
}
```

---

## MockMpd Implementation

### Data Structure

```rust
pub struct MockMpd {
    playlists: Arc<Mutex<HashMap<String, Vec<Song>>>>,
    queue: Arc<Mutex<Vec<Song>>>,
    is_consuming: Arc<Mutex<bool>>,
    connection_state: Arc<Mutex<bool>>,
    // Added for testing:
    pushed_history: Arc<Mutex<Vec<Song>>>,  // Track what was pushed
}
```

### Key Methods

```rust
impl MockMpd {
    pub fn new() -> Self
    
    pub fn add_playlist(&self, name: &str, songs: Vec<Song>)
    
    pub fn simulate_disconnect(&self)
    pub fn simulate_reconnect(&self)
    
    pub fn clear_state(&self)  // For test isolation
    pub fn get_pushed_history(&self) -> Vec<Song>  // For verification
}
```

---

## Isolated Testing Pattern

### Creating Isolated Mocks

**CORRECT** - Each test gets fresh mock:
```rust
#[test]
fn test_something() {
    let mock = MockMpd::new();
    mock.add_playlist("tag_a);
    
    let", songs mpd_conn = MpdConn::new_for_testing(mock);
    // Test...
}
```

### MpdConn::new_for_testing()

```rust
// In server/src/mpd_conn/mpd_conn.rs
impl MpdConn {
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

## What MockMpd Must Implement

### For Album-Aware Shuffle Tests

| Method | Required | Notes |
|--------|----------|-------|
| `search()` | Yes | Can return all songs; caller filters |
| `playlist()` | Yes | Return songs from a playlist |
| `playlists()` | Yes | Return list of playlist names |
| `queue()` | Yes | Return current queue |
| `push()` | Yes | Add to queue |
| `delete()` | Yes | Remove from queue |

### For Scheduler Tests

| Method | Required | Notes |
|--------|----------|-------|
| All above | Yes | |
| `clear_state()` | Yes | Reset between tests |
| `get_pushed_history()` | Yes | Verify what was added |

---

## What MockMpd Does NOT Need

### search() Complexity

**MockMpd does NOT need to parse MPD queries.** It can return ALL songs and let the caller filter.

```rust
// Simple mock search - acceptable!
fn search(&mut self, query: &Query, ...) -> Result<Vec<Song>> {
    // Return all songs; caller (SongQueue) filters
    let playlists = self.playlists.lock().unwrap();
    let mut results = Vec::new();
    for playlist in playlists.values() {
        results.extend(playlist.clone());
    }
    Ok(results)
}
```

This is acceptable because:
1. SongQueue already performs exact filtering
2. Tests verify end behavior, not intermediate filtering

---

## Common Mistakes

### ❌ Wrong: Shared Static Mock

```rust
// BAD - causes test pollution
static SHARED_MOCK: OnceLock<MockMpd> = OnceLock::new();

impl MpdConn {
    pub fn new() -> Result<Self> {
        let mock = SHARED_MOCK.get_or_init(MockMpd::new).clone();
        // All tests share same mock!
    }
}
```

**Problem**: Tests interfere with each other. A test that adds playlist "tag_a" will pollute tests that expect only "tag_b".

### ✅ Correct: Isolated Mocks

```rust
// GOOD - each test gets fresh mock
let mock = MockMpd::new();
let mpd_conn = MpdConn::new_for_testing(mock);
```

---

## Environment Variable

```bash
export JUKECTL_DEV_MODE=1
```

When set, `MpdConn::new()` returns a Mock-backed connection instead of connecting to real MPD.

---

## Test Commands

```bash
# Run album-aware tests
cargo test --test album_aware_shuffle_test

# Run scheduler/simulator tests  
cargo test --test simulator

# Run all tests
cargo test
```

---

## Related Specs

- [SPEC 0001: Vinyl Jukebox Philosophy](./0001-vinyl-jukebox.md)
- [SPEC 0002: Album-Aware Shuffle](./0002-album-aware-shuffle.md)
- [SPEC 0003: Behavioral Simulator](./0003-behavioral-simulator.md)

---

## Changelog

- 2026-03-05: Initial spec - neoice
