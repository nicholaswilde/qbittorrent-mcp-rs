# Search Functionality Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `SearchResults` and `SearchStatus` structs in `src/models.rs`.
- [x] Task: Implement Search Methods in `QBitClient`
    - [x] `start_search(pattern, category)` -> returns ID.
    - [x] `get_search_results(id, limit, offset)` -> returns Vec<Result>.
    - [x] `stop_search(id)`.
    - [x] `delete_search(id)`.
- [x] Task: Integration Test (Mocked)
    - [x] Create `tests/client_search_test.rs` to verify the "start -> poll -> stop" flow.

## Phase 2: MCP Tool Integration
- [x] Task: Implement `search_torrents` Tool Logic
    - [x] In `src/server/handler.rs`, implement a helper function `perform_search` that handles the polling loop.
    - [x] Register `search_torrents` in `tools/list` (respecting lazy mode).
    - [x] Handle `search_torrents` in `tools/call`.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.