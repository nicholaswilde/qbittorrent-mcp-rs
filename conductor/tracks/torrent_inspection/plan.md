# Torrent Inspection Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `TorrentFile` struct in `src/models.rs`.
    - [x] Create `TorrentProperties` struct in `src/models.rs`.
- [x] Task: Implement Methods in `QBitClient`
    - [x] Add `get_torrent_files(hash)` to `src/client.rs`.
    - [x] Add `get_torrent_properties(hash)` to `src/client.rs`.
- [x] Task: Unit Tests
    - [x] Create `tests/client_inspection_test.rs` to mock and verify these calls.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] Update `src/server/handler.rs` to include `get_torrent_files` and `get_torrent_properties` in `tools/list`.
    - [x] Implement handler logic in `tools/call`.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.
    - [x] Verified against real instance (Properties and Files retrieval working).
