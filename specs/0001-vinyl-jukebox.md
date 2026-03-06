---
title: The Vinyl Jukebox Philosophy
lifecycle: building
status: [CURRENT]
author: neoice
created: 2026-03-05
---

# SPEC 0001: The Vinyl Jukebox Philosophy

## Why This Document Exists

This spec defines the **core philosophical model** of jukectl. Without understanding this model, developers and AI agents will misunderstand the system's purpose and implement features that contradict its intent.

---

## The Core Metaphor

**jukectl is a virtual vinyl jukebox.**

When you select a song in album-aware mode, you're not queuing a song—you're dropping a record on a turntable, setting the needle to 0:00 on track 1, and letting it play end-to-end, side-to-side, vinyl-to-vinyl if required.

This is not "shuffle." This is **album-aware playback**.

---

## Behavioral Invariants

These invariants must NEVER be violated:

### 1. Atomic Album Delivery

**Rule**: When `dequeue()` is called in album-aware mode, the return is ALL songs from that album—not one.

**Why**: Because dropping a record means playing the whole record. You don't lift the needle after track 1.

**Implementation**:
- The queue contains "seeds" (one song per album)
- `dequeue()` expands a seed into the full album
- The caller receives N songs (where N = album track count)

### 2. Track Number Ordering

**Rule**: Songs within an album are ALWAYS returned sorted by Track number (1, 2, 3...).

**Why**: Because that's how vinyl works. Track 1 plays before Track 2.

**Implementation**:
- After expanding the album, sort by `Track` tag
- Tracks without a number sort first (default: 0)

### 3. Album Identity (AlbumArtist > Artist)

**Rule**: Album identity is determined by `AlbumArtist` tag. If absent, fall back to `Artist`.

**Why**: Multiple artists can have albums with the same name (e.g., "Greatest Hits"). We must distinguish them.

**Implementation**:
```
seed.album == other.album 
AND (seed.album_artist == other.album_artist OR (both missing AND seed.artist == other.artist))
```

### 4. No Album Duplication

**Rule**: Once an album is dequeued, it is removed from the queue. No album appears twice until all albums are exhausted.

**Why**: You've already heard the record. You don't drop the same vinyl twice in one session.

**Implementation**:
- Queue stores seeds, not individual songs
- Each `dequeue()` removes exactly 1 seed (regardless of album size)
- When queue is empty, shuffle and refill with new seeds

### 5. Exact Album Match

**Rule**: Album matching must be exact string match, not substring.

**Why**: "Greatest Hits" should NOT match "Greatest Hits 1999" or "Greatest Hits (Remastered)".

---

## The Two Modes

### Regular Mode (album_aware = false)

- Queue contains individual songs
- `dequeue()` returns 1 song
- Traditional shuffle behavior

### Album-Aware Mode (album_aware = true)

- Queue contains album seeds
- `dequeue()` returns N songs (full album)
- Vinyl behavior

---

## What "Shuffle" Means

In album-aware mode, "shuffle" operates on **albums**, not songs:

1. Query all songs matching current tags
2. Group by (Album, AlbumArtist/Artist)
3. From each album group, select 1 seed song (randomly or by popularity)
4. Shuffle the list of seeds
5. Place seeds in queue

The shuffle is **fair**—over time, all albums should appear with roughly equal probability.

---

## Scheduler Behavior

The scheduler runs a tick every 3 seconds:

1. Check MPD queue length
2. If MPD queue < 2 songs, call `dequeue()` to get more
3. Push returned songs to MPD queue
4. If internal queue is empty (len == 0), refill from tags

**Critical**: Refill happens at `len == 0`, not `len == 1`. This is because MPD consume mode removes songs as they play.

---

## Testing Philosophy

**We test behavior, not implementation.**

For album-aware shuffle, we test:
- Does dequeue return the full album? (not 1 song)
- Are tracks sorted by track number? (not filename)
- Does the same album appear twice? (should not)
- Does track number sorting handle missing tracks? (yes, defaults to 0)

We do NOT test:
- Internal cache implementation details
- Specific shuffle algorithm (unless it's observably biased)

---

## Related Specs

- [SPEC 0002: Album-Aware Shuffle Implementation](./0002-album-aware-shuffle.md)
- [SPEC 0003: Behavioral Simulator](./0003-behavioral-simulator.md)
- [SPEC 0004: Mock Infrastructure](./0004-mock-infrastructure.md)

---

## Changelog

- 2026-03-05: Initial spec - neoice
