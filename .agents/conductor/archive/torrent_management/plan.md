# Torrent Management Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Implement `add_torrent` in `QBitClient`
    - [x] Update `src/client.rs` to add `add_torrent` method.
    - [x] Create `tests/client_management_test.rs` to test `add_torrent` (mocked).
- [x] Task: Implement Control Methods in `QBitClient`
    - [x] Update `src/client.rs` to add `pause_torrents`, `resume_torrents`, `delete_torrents`.
    - [x] Update `tests/client_management_test.rs` to test these methods (mocked).

## Phase 2: MCP Tool Integration
- [x] Task: Create Tool Definitions
    - [x] Update `src/lib.rs` or `src/server/mod.rs` (wherever tools are defined) to include structs/logic for the new tools.
    - [x] Implement `Tool` trait or handler logic for:
        - `add_torrent`
        - `pause_torrent`
        - `resume_torrent`
        - `delete_torrent`
- [x] Task: Register Tools
    - [x] Register the new tools in the main loop/server definition so they are exposed to the client.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test` to ensure all mocks pass.
    - [x] Run `cargo build` to ensure no compile errors.
    - [x] Verified against real instance (List, Add, Pause, Resume, Delete confirmed working).