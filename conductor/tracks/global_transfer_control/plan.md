# Global Transfer Control Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `TransferInfo` struct in `src/models.rs`.
- [x] Task: Implement Methods in `QBitClient`
    - [x] Add `get_global_transfer_info()` to `src/client.rs`.
    - [x] Add `set_download_limit(limit)` to `src/client.rs`.
    - [x] Add `set_upload_limit(limit)` to `src/client.rs`.
- [x] Task: Unit Tests
    - [x] Create `tests/client_transfer_test.rs` to mock and verify these calls.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] Update `src/server/handler.rs` to include `get_global_transfer_info` and `set_global_transfer_limits` in `tools/list`.
    - [x] Implement handler logic in `tools/call`.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.