# Jules Engineering Guide: How to Dispatch Work

This document is a reference for humans and other AI agents on how to effectively delegate tasks to **Google Jules**. Jules is an autonomous coding agent that works best when provided with clear verification loops and deterministic environments.

## 🧠 Jules' Mental Model

- **Asynchronous & Independent**: Jules doesn't just suggest code; it plans, executes, and verifies. It operates best when it can "disappear" into a task and return with a completed PR.
- **Verification-First**: Jules will prioritize running tests. If tests are hard to run or require external hardware (like a real MPD server), Jules will likely get stuck or hallucinate success.
- **Implementation Plans**: Jules always generates a plan for approval. It excels at tasks where the "How" has multiple steps across multiple files.

## 💪 What Jules Does Best

1. **Infrastructure & Harnessing**: Building more mocks, adding test coverage, and creating simulation environments.
2. **Feature Porting**: Taking a well-defined feature from a spec (or another language) and implementing it idiomatically in Rust.
3. **Refactoring**: Standardizing patterns (e.g., "Ensure all routes use the same error handling wrapper").
4. **Maintenance**: Upgrading dependencies, fixing Clippy lints, and updating documentation.

## ⚠️ Known Quirks & Gotchas

- **The "Hardware Wall"**: Jules cannot "hear" music or see an MPD server. **Never** ask Jules to test against a real MPD instance. Always point it to `JUKECTL_DEV_MODE=1`.
- **Vague Boundaries**: If a task is "Make the jukebox better," Jules will struggle. If the task is "Implement a 'History' endpoint that tracks the last 50 played songs in an in-memory buffer," it will thrive.
- **Personal Preference**: Jules is a "correct" engineer. It won't know your "Highly Opinionated" vibes unless you state them (e.g., "Keep the API lean," "Unix philosophy only").

## 📋 Task Authoring Template (For Agents/Humans)

When creating a GitHub issue for Jules, use this structure:

```markdown
## Goal
[Clear, one-sentence description of the outcome]

## Context
- Logic lives in: `server/src/...`
- MPD interaction: Use `MpdClient` trait (do not use raw crates).
- Dev Mode: Ensure it works with `JUKECTL_DEV_MODE=1`.

## Acceptance Criteria
- [ ] Feature X implemented.
- [ ] New test file `server/tests/...` added.
- [ ] `cargo check` and `cargo test` pass in dev mode.

## Hints
- See `GEMINI.md` for repo-specific rules.
- The `MockMpd` needs to be updated if you add new trait methods.
```

## 🚀 How to Dispatch
1. Ensure the repo has a clean `MockMpd` state.
2. Create a GitHub Issue with the template above.
3. **Apply the `jules` label.**
4. Monitor the implementation plan Jules provides.
