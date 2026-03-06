---
title: Album-Aware Shuffle Implementation
lifecycle: building
status: [CURRENT]
author: neoice
created: 2026-03-05
depends_on:
  - 0001-vinyl-jukebox
---

# SPEC 0002: Album-Aware Shuffle Implementation

## Overview

This spec defines the implementation of album-aware shuffle—the vinyl jukebox behavior where playback flows through entire albums in track order.

**Read [SPEC 0001](./0001-vinyl-jukebox.md) first** to understand the behavioral invariants.

---

## Current Implementation

### Code Location

- **Core logic**: `server/src/models/song_queue.rs`
- **Tests**: `server/tests/album_aware_shuffle_test.rs`

### Key Data Structures

```rust
pub struct SongQueue {
    inner: VecDeque<mpd::Song>,  // Queue of "seeds" (one song per album)
    album_aware: bool,             // Mode flag
    // ... caching and performance tracking
}
```

### Key Methods

```rust
// Mode control
pub fn set_album_aware(&mut self, album_aware: bool)

// Main entry point - returns songs to add to MPD queue
pub fn dequeue(&mut self, mpd_client: &mut MpdConn) -> Vec<mpd::Song> {
    if self.album_aware {
        self.dequeue_as_album(mpd_client)
    } else {
        self.dequeue_single()
    }
}

// Album-aware: expands seed to full album
fn dequeue_as_album(&mut self, mpd_client: &mut MpdConn) -> Vec<mpd::Song>

// Regular: returns one song
pub fn dequeue_single(&mut self) -> Vec<mpd::Song>
```

---

## The Algorithm: dequeue_as_album

```
1. Pop first seed song from queue
   IF queue empty, RETURN []

2. Extract album name from seed's "Album" tag
   Extract artist from seed's "AlbumArtist" OR "Artist" tag

3. Query MPD for ALL songs matching album name
   (search by "album" tag)

4. Filter results:
   - Must have exact album name match
   - Must match artist (if seed had artist info)
   - Must be unique by file path

5. Sort filtered songs by Track number (ascending)
   - Songs without Track tag: sort first (default 0)

6. RETURN sorted songs (typically N songs where N = album track count)
```

---

## Critical Implementation Details

### Artist Matching Logic

```rust
// From song_queue.rs:219-228
let artist_matches = if let Some(ref artist) = seed_artist {
    // Check both AlbumArtist and Artist tags
    Self::get_tag_value(song, "AlbumArtist")
        .or_else(|| Self::get_tag_value(song, "Artist"))
        .map(|a| a == *artist)
        .unwrap_or(false)
} else {
    // No artist info in seed, don't filter by artist
    true
};
```

**This means**:
- If seed has artist info → filter by artist
- If seed has NO artist info → return ALL songs from that album (useful for compilations)

### Track Sorting

```rust
// From song_queue.rs:246-254
sorted_songs.sort_by(|a, b| {
    let track_a = Self::get_tag_value(a, "Track")
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(0);
    let track_b = Self::get_tag_value(b, "Track")
        .and_then(|t| t.parse::<u32>().ok())
        .unwrap_or(0);
    track_a.cmp(&track_b)
});
```

---

## Canonical Test Fixture: Realistic Music Library

**Problem**: Fake data finds fake bugs. Tests with "Album 1", "Album 2" don't expose real edge cases.

**Solution**: A realistic test fixture reflecting actual music library messiness.

### The Fixture

Create `server/tests/fixtures/realish_library.rs`:

```rust
use mpd::Song;

/// A realistic test music library reflecting years of accumulated metadata chaos.
/// This fixture should feel like a real music collection - messy, inconsistent, real.
pub fn realish_library() -> Vec<Song> {
    vec![
        // ============================================
        // NORMAL ALBUMS - Well-organized
        // ============================================
        
        // Pink Floyd - Dark Side of the Moon (canonical rock album)
        mk_song("music/pink_floyd/dark_side/01_speak_to_me.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Speak to Me", 1),
        mk_song("music/pink_floyd/dark_side/02_breathe.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Breathe", 2),
        mk_song("music/pink_floyd/dark_side/03_on_the_run.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "On the Run", 3),
        mk_song("music/pink_floyd/dark_side/04_time.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Time", 4),
        mk_song("music/pink_floyd/dark_side/05_the_great_gig_in_the_sky.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "The Great Gig in the Sky", 5),
        mk_song("music/pink_floyd/dark_side/06_money.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Money", 6),
        mk_song("music/pink_floyd/dark_side/07_us_and_them.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Us and Them", 7),
        mk_song("music/pink_floyd/dark_side/08_any_colour_you_like.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Any Colour You Like", 8),
        mk_song("music/pink_floyd/dark_side/09_brain_damage.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Brain Damage", 9),
        mk_song("music/pink_floyd/dark_side/10_eclipse.mp3", 
                "Pink Floyd", "Dark Side of the Moon", "Eclipse", 10),

        // Led Zeppelin IV (no track numbers in folder names)
        mk_song("music/led_zeppelin/iv/black_dog.mp3", 
                "Led Zeppelin", "Led Zeppelin IV", "Black Dog", 1),
        mk_song("music/led_zeppelin/iv/rock_and_roll.mp3", 
                "Led Zeppelin", "Led Zeppelin IV", "Rock and Roll", 3),  // Track 2 is missing!
        mk_song("music/led_zeppelin/iv/stairway_to_heaven.mp3", 
                "Led Zeppelin", "Led Zeppelin IV", "Stairway to Heaven", 5),

        // ============================================
        // COMPILATIONS - The messiest category
        // ============================================
        
        // NOW That's What I Call Music! series
        mk_song("music/compilations/now01/01_adele_hello.mp3", 
                "Adele", "Now That's What I Call Music! 63", "Hello", 1),
        mk_song("music/compilations/now01/02_ed_sheeran_shape.mp3", 
                "Ed Sheeran", "Now That's What I Call Music! 63", "Shape of You", 2),
        
        // Another NOW album
        mk_song("music/compilations/now02/01_drake_hotline_bling.mp3", 
                "Drake", "Now That's What I Call Music! 64", "Hotline Bling", 1),
        
        // Soundtracks with multiple artists
        mk_song("music/soundtracks/guardians_of_the_galaxy/01_hookman.mp3", 
                "Blue Swede", "Guardians of the Galaxy: Awesome Mix Vol. 1", "Hooked on a Feeling", 1),
        mk_song("music/soundtracks/guardians_of_the_galaxy/02_go_all_the_way.mp3", 
                "The Raspberries", "Guardians of the Galaxy: Awesome Mix Vol. 1", "Go All the Way", 2),

        // ============================================
        // SAME ALBUM NAME, DIFFERENT ARTISTS
        // This is a CRITICAL edge case
        // ============================================
        
        mk_song("music/the_band/greatest_hits/01_up_on_cripple_creek.mp3", 
                "The Band", "Greatest Hits", "Up on Cripple Creek", 1),
        mk_song("music/the_band/greatest_hits/02_the_weight.mp3", 
                "The Band", "Greatest Hits", "The Weight", 2),
        
        mk_song("music/different_band/greatest_hits/01_another_song.mp3", 
                "Different Band", "Greatest Hits", "Another Song", 1),
        mk_song("music/different_band/greatest_hits/02_yet_another.mp3", 
                "Different Band", "Greatest Hits", "Yet Another", 2),

        // ============================================
        // REMASTERS / REISSUES
        // Album same, but technically different releases
        // ============================================
        
        mk_song("music/miles_davis/kind_of_blue/original/01_so_what.mp3", 
                "Miles Davis", "Kind of Blue (Original)", "So What", 1),
        mk_song("music/miles_davis/kind_of_blue/remastered/01_so_what.mp3", 
                "Miles Davis", "Kind of Blue (Remastered)", "So What", 1),

        // ============================================
        // LIVE ALBUMS
        // ============================================
        
        mk_song("music/led_zeppelin/live_rah/01_whole_lotta_love.mp3", 
                "Led Zeppelin", "Live at Royal Albert Hall", "Whole Lotta Love", 1),
        mk_song("music/led_zeppelin/live_rah/02_immigrant_song.mp3", 
                "Led Zeppelin", "Live at Royal Albert Hall", "Immigrant Song", 2),

        // ============================================
        // COLLABORATIONS / FEATURING
        // ============================================
        
        mk_song("music/various/clean_bandit_symphony/01_rockabye.mp3", 
                "Clean Bandit feat. Sean Paul", "Symphony", "Rockabye", 1),
        mk_song("music/various/clean_bandit_symphony/02_symphony.mp3", 
                "Clean Bandit feat. Zara Larsson", "Symphony", "Symphony", 2),

        // ============================================
        // CLASSICAL / ORCHESTRAL
        // Different naming conventions
        // ============================================
        
        mk_song("music/beethoven/symphony_9/choral/01_Allegro.mp3", 
                "Ludwig van Beethoven", "Symphony No. 9", "I. Allegro ma non troppo", 1),
        mk_song("music/beethoven/symphony_9/choral/02_Molto_allegro.mp3", 
                "Ludwig van Beethoven", "Symphony No. 9", "II. Molto allegro", 2),
        
        // Different conductor = different album often
        mk_song("music/furtwangler/beethoven_9/01_Allegro.mp3", 
                "Furtwangler", "Beethoven: Symphony No. 9", "I. Allegro", 1),

        // ============================================
        // MISSING / INCOMPLETE METADATA
        // The reality of old CD rips
        // ============================================
        
        // No artist
        mk_song("music/unknown/01_unknown_track.mp3", 
                "", "Unknown Album", "Unknown Track 1", 1),
        mk_song("music/unknown/02_unknown_track.mp3", 
                "", "Unknown Album", "Unknown Track 2", 2),
        
        // No track number (CD rip from ancient times)
        mk_song("music/old_cd_rips/artist_album/track_name.mp3", 
                "Old Artist", "Old Album", "Track Without Number", None),
        
        // Track number as string "1" vs number 1
        mk_song("music/inconsistent/track_1.mp3", 
                "Artist", "Album", "Track One", 1),
        mk_song("music/inconsistent/track_2.mp3", 
                "Artist", "Album", "Track Two", 2),

        // ============================================
        // VA - ALBUMARTIST TAG (for compilations)
        // AlbumArtist = "Various Artists" means return ALL
        // ============================================
        
        mk_song("music/va/movie_soundtrack/01_tracy_chapman_fast_car.mp3", 
                "Tracy Chapman", "Best of Movie Soundtracks", "Fast Car", 1),
        mk_song("music/va/movie_soundtrack/02_wham_careless_whisper.mp3", 
                "Wham!", "Best of Movie Soundtracks", "Careless Whisper", 2),
    ]
}

/// Helper to create test songs consistently
fn mk_song(path: &str, artist: &str, album: &str, title: &str, track: Option<u32>) -> Song {
    let mut song = Song::default();
    song.file = path.to_string();
    song.tags = vec![
        ("Artist".to_string(), artist.to_string()),
        ("Album".to_string(), album.to_string()),
        ("Title".to_string(), title.to_string()),
    ];
    if let Some(t) = track {
        song.tags.push(("Track".to_string(), t.to_string()));
    }
    song
}
```

### How to Use This Fixture

```rust
#[test]
fn test_vinyl_drop_with_real_data() {
    let mock = MockMpd::new();
    
    // Load the REALISH library into the mock
    let songs = realish_library();
    mock.add_playlist("test_tag", songs);
    
    let mpd_conn = MpdConn::new_for_testing(mock);
    let mut queue = SongQueue::new();
    queue.set_album_aware(true);
    
    // Seed with a Pink Floyd song
    let pink_floyd_song = find_song_by_title(&queue, "Speak to Me").unwrap();
    queue.add(pink_floyd_song);
    
    // Dequeue - should return ALL 10 Dark Side tracks
    let results = queue.dequeue(&mut mpd_conn);
    
    assert_eq!(results.len(), 10);  // Full album!
    assert_eq!(results[0].file, "music/pink_floyd/dark_side/01_speak_to_me.mp3");
    assert_eq!(results[9].file, "music/pink_floyd/dark_side/10_eclipse.mp3");
}

#[test]
fn test_compilation_returns_all() {
    // AlbumArtist empty means VA compilation - return ALL songs from album
    let mock = MockMpd::new();
    let songs = realish_library();
    mock.add_playlist("test_tag", songs);
    
    // ... test that compilations return all tracks
}
```

---

## Test Scenarios (What Must Pass)

### Scenario A: The Vinyl Drop
- **Behavior**: dequeue returns ENTIRE album (not 1 song)
- **Test**: Seed with 4-track album, verify dequeue returns 4 songs

### Scenario B: Complete Playthrough
- **Behavior**: 10 albums, exhaust all, verify no duplicates
- **Test**: 10 albums × 3-8 tracks each, dequeue until empty

### Scenario C: Track Number Sorting
- **Behavior**: Tracks returned in 1→2→3 order
- **Test**: Shuffle track order in MPD, verify sorted output

### Scenario D: Missing Track Numbers
- **Behavior**: Handle gracefully
- **Test**: Album with no Track tags

### Scenario E: AlbumArtist vs Artist
- **Behavior**: Different artists, same album name = different albums
- **Test**: "Greatest Hits" by Band A vs Band B

### Scenario F: Single Album
- **Behavior**: Edge case works
- **Test**: Library with 1 album

### Scenario G: Distribution Fairness
- **Behavior**: No album dominates
- **Test**: Statistical: 10 albums, 100 shuffles, max 25% per album

### Scenario H: Empty Album
- **Behavior**: Return seed song, log warning
- **Test**: Seed album returns empty from MPD

### Scenario I: Unknown Artist
- **Behavior**: Group by Album only
- **Test**: Album with no Artist tag

### Scenario J: Compilation Albums
- **Behavior**: Return ALL tracks (Various Artists)
- **Test**: Album="Soundtrack", multiple artists

### Scenario K: Mode Toggle
- **Behavior**: Switching modes doesn't corrupt state
- **Test**: album-aware → regular → album-aware

### Scenario L: Queue State
- **Behavior**: Queue decreases by 1 (seed), not album size
- **Test**: Verify internal queue length after dequeue

---

## Common Mistakes to Avoid

### ❌ Wrong: Return 1 song in album-aware mode
```rust
// THIS IS WRONG
fn dequeue_as_album(&mut self, ...) -> Vec<mpd::Song> {
    vec![self.inner.pop_front().unwrap()]  // Only 1 song!
}
```

### ✅ Correct: Expand to full album
```rust
// THIS IS CORRECT
fn dequeue_as_album(&mut self, mpd_client: &mut MpdConn) -> Vec<mpd::Song> {
    let seed = self.inner.pop_front().unwrap();
    // Query MPD, filter by album, sort by track...
    album_songs  // Returns N songs
}
```

### ❌ Wrong: Substring album matching
```rust
// THIS IS WRONG - "Hits" matches "Greatest Hits"
songs.iter().filter(|s| s.album.contains(album_name))
```

### ✅ Correct: Exact album matching
```rust
// THIS IS CORRECT
songs.iter().filter(|s| s.album == album_name)
```

---

## Mock Requirements

For tests to work, `MockMpd` must:

1. **Support `search()` method** - Returns songs matching query
2. **Support `playlist()` method** - Returns songs in a playlist
3. **Isolated instances** - Each test needs its own mock (no shared state)

**IMPORTANT**: MockMpd does NOT need to implement complex query parsing. It can return ALL songs and let the caller (SongQueue) filter. This is acceptable because:
- SongQueue already performs exact filtering
- Tests verify the end behavior, not the query implementation

---

## Related Specs

- [SPEC 0001: Vinyl Jukebox Philosophy](./0001-vinyl-jukebox.md)
- [SPEC 0003: Behavioral Simulator](./0003-behavioral-simulator.md)
- [SPEC 0004: Mock Infrastructure](./0004-mock-infrastructure.md)

---

## Changelog

- 2026-03-05: Initial spec - neoice
