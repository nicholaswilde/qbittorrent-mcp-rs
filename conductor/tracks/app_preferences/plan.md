# App Preferences Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Implement Methods in `QBitClient`
    - [x] `get_app_preferences()` -> returns `serde_json::Value`.
    - [x] `set_app_preferences(json_value)` -> accepts `serde_json::Value`.
- [x] Task: Unit Tests
    - [x] Create `tests/client_app_test.rs` to mock and verify these calls.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] Update `src/server/handler.rs` to include `get_app_preferences` and `set_app_preferences`.
    - [x] Implement handler logic in `tools/call`.
    - [x] Hide in lazy mode by default.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.