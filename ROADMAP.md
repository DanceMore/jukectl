# jukectl Roadmap

This document outlines the strategic priorities for `jukectl` and serves as a backlog for agentic engineering tasks.

## 🎯 Priority 1: Realistic Behavioral Testing (Harness Engineering)
Currently, our tests are mostly unit-based. We need a "Jukebox Simulator" suite that uses `MockMpd` to verify long-running jukebox behaviors.

- [ ] **Task**: Create `server/tests/jukebox_simulator.rs`.
- [ ] **Scenario**: A "Busy Night" scenario where the queue is constantly being drained and refilled.
- [ ] **Scenario**: "Tag Conflict" scenario where tags are changed while the scheduler is mid-cycle.
- [ ] **Requirement**: Use `JUKECTL_DEV_MODE=1` to ensure these can run in any CI environment without a real MPD.

## 📊 Priority 2: Popularity-Sorted Tag Cheatsheet
The `jukectl tags` command should help the user find their "best" music quickly.

- [x] **Implementation**: Add track counts to the available tags listing. (In progress)
- [ ] **Task**: Implement usage tracking. (Future)
- [ ] **Requirement**: Sort the CLI output so the tags with the most tracks (the "popular" ones) appear at the top.

## 🛠️ Priority 3: Library & Janitor Services
- [ ] **Task**: Create a "Janitor" service that periodically triggers an MPD `update` if files change on disk.
- [ ] **Task**: Add a `jukectl health` command to verify MPD connection health and library status.

## 📡 Priority 4: Modernized Configuration
- [ ] **Task**: Support loading default tags from a configuration file or a Base64-encoded environment variable.
- [ ] **Task**: Implement a "Party Mode" toggle that prevents `skip` or `untag` operations for a set duration.
