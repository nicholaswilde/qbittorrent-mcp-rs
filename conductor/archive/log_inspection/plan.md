# System Log Inspection Implementation Plan

## Phase 1: Client Implementation
- [ ] Task: Define Models
    - [ ] Create `LogEntry` struct in `src/models.rs`.
- [ ] Task: Implement Methods in `QBitClient`
    - [ ] `get_main_log(normal, info, warning, critical, last_id)`
    - [ ] `get_peer_log(last_id)`
- [ ] Task: Unit Tests
    - [ ] Create `tests/client_log_test.rs`.

## Phase 2: MCP Tool Integration
- [ ] Task: Register Tools
    - [ ] Update `src/server/handler.rs` to include `get_main_log` and `get_peer_log`.
    - [ ] Implement handler logic in `tools/call`.
    - [ ] Hide in lazy mode by default.

## Phase 3: Verification
- [ ] Task: Verify Compilation and Tests
    - [ ] Run `cargo test`.
    - [ ] Build project.
