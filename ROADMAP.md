# jukectl Roadmap & Feature Corpus

This document outlines strategic priorities and provides **High-Fidelity Specifications** for agentic engineering (specifically for Jules).

---

## 🎯 Priority 1: Jukebox Behavioral Simulator
**Status**: Ready for Dispatch (Spec Below)

### Jules Spec: Behavioral Simulator Suite
- **Goal**: Create a robust suite of integration tests that simulate long-running jukebox scenarios.
- **Context**: Use the `MpdClient` trait and `MockMpd`. Execute with `JUKECTL_DEV_MODE=1`.
- **Requirements**:
    - Create `server/tests/simulator.rs`.
    - **Scenario A (Refill Logic)**: Seed the mock with 10 songs. Drain the queue to 1 song. Verify the scheduler automatically refills the queue using the current tags.
    - **Scenario B (Tag Hot-Swap)**: Change active tags while the scheduler is sleeping. Verify the next refill uses the *new* tags.
- **Acceptance Criteria**: Tests must pass consistently without a real MPD server.

---

## 📊 Priority 2: History & Usage Tracking
**Status**: Scoping (Spec Below)

### Jules Spec: In-Memory Playback History
- **Goal**: Track the last 50 songs played by the jukebox and expose them via an API.
- **Context**:
    - Add a `History` struct to `AppState`.
    - Update `scheduler/mod.rs` to push the "Now Playing" song into history whenever the queue advances.
- **Requirements**:
    - New Endpoint: `GET /history` returning a JSON list of song filenames.
    - CLI Command: `jukectl history` to display the list.
- **Acceptance Criteria**: History survives until the process restarts. No duplicates if the same song repeats.

---

## 🛠️ Priority 3: Library Janitor (Auto-Update)
**Status**: Backlog

### Jules Spec: MPD Database Sync
- **Goal**: Add an autonomous service that ensures the MPD database is updated when files change.
- **Context**: MPD has an `update` command.
- **Requirements**:
    - Add a periodic task (every 5 mins) in `scheduler/mod.rs` that calls `mpd_client.update()`.
    - Add a CLI command `jukectl sync` to trigger this manually.
- **Acceptance Criteria**: Uses the `MpdClient` trait (add `update` method to trait if missing).

---

## 📡 Priority 4: Modernized Configuration
**Status**: Backlog

### Jules Spec: Default Tags from Env
- **Goal**: Allow setting the default "boot" tags via a Base64-encoded environment variable.
- **Context**: Currently hardcoded in `app_state.rs`.
- **Requirement**: If `JUKECTL_DEFAULT_TAGS_B64` is present, decode it and use it to initialize `TagsData`.
- **Acceptance Criteria**: Graceful fallback to "jukebox" tag if decoding fails.
