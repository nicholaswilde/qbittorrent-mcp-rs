# Utility Tools Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Implement Specific Torrent Info Method
    - [x] Update `QBitClient` to support fetching info for a list of hashes (useful for polling).
- [x] Task: Unit Tests
    - [x] Mock the hash-specific info response.

## Phase 2: MCP Tool Integration
- [x] Task: Implement `wait_for_torrent_status` Tool Logic
    - [x] In `src/server/handler.rs`, implement the polling loop with `tokio::time::sleep`.
    - [x] Register tool in `tools/list`.
    - [x] Handle tool call in `tools/call`.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.