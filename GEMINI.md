# jukectl - Agentic Engineering Guide

This document provides instructions for AI agents (like Google Jules, Gemini CLI, etc.) on how to build, test, and evolve the `jukectl` codebase.

## Repository Structure

- `server/`: The backend jukebox daemon (Rust + Rocket).
- `cli/`: The command-line interface (Rust + Clap).
- `common/`: Shared models and utilities.

## Local Development & Mocking

`jukectl` is designed to be developed without a real MPD (Music Player Daemon) instance. 

- **Dispatching to Jules**: See [JULES_GUIDE.md](JULES_GUIDE.md) for tips on how to effectively assign tasks to Google Jules.
- **Active Backlog**: See [ROADMAP.md](ROADMAP.md) for high-fidelity task specifications ready for implementation.

### JUKECTL_DEV_MODE
To run the server in a purely virtual environment with an in-memory MPD mock:
```bash
export JUKECTL_DEV_MODE=1
cd server && cargo run
```
In this mode, the server will not attempt to connect to a real MPD socket. It uses `MockMpd` which tracks state in memory.

## Build & Test Commands

### Server
```bash
cd server
cargo build
cargo test
cargo clippy
```

### CLI
```bash
cd cli
cargo build
cargo test
```

## Agent Guidelines

1. **Prefer Mocks**: When adding new features to the server, ensure they are reflected in the `MpdClient` trait (`server/src/mpd_conn/traits.rs`) and implemented in `MockMpd` (`server/src/mpd_conn/mock_mpd.rs`).
2. **Tag Rules**: Valid jukebox tags are represented by MPD playlists. However, any playlist matching the pattern `yyyy-mm` or `yyyy-mm-dd` is considered an **ad-hoc playlist** and is filtered out by the core engine.
3. **Async Awareness**: The server uses `tokio` and `rocket`. Ensure any new blocking operations are handled appropriately.
4. **Connection Pooling**: Use the `MpdConnectionPool` provided in `AppState` rather than creating raw connections.
