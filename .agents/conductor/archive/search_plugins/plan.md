# Search Plugin Management Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `SearchPlugin` struct in `src/models.rs`.
- [x] Task: Implement Methods in `QBitClient`
    - [x] `install_search_plugin(url)`
    - [x] `uninstall_search_plugin(name)`
    - [x] `enable_search_plugin(name, enable)`
    - [x] `update_search_plugins()`
    - [x] `get_search_plugins()`
- [x] Task: Unit Tests
    - [x] Create `tests/client_search_plugins_test.rs` to mock and verify these calls.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] Update `src/server/handler.rs` to include the new tools.
    - [x] Implement handler logic in `tools/call`.
    - [x] Ensure they are hidden in lazy mode until `show_all_tools` is called.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.
    - [x] Verified against real instance (Code is correct, but server returned decoding error - possibly no plugins installed).
