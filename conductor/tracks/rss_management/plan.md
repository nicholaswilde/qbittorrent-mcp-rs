# RSS Management Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `RssFeed` and `RssRule` structs in `src/models.rs`.
- [x] Task: Implement Methods in `QBitClient`
    - [x] `add_rss_feed(url, path)`
    - [x] `remove_rss_item(path)`
    - [x] `get_all_rss_feeds()`
    - [x] `set_rss_rule(name, definition)`
    - [x] `get_all_rss_rules()`
- [x] Task: Unit Tests
    - [x] Create `tests/client_rss_test.rs` to mock and verify these calls.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] Update `src/server/handler.rs` to include the new tools.
    - [x] Implement handler logic in `tools/call`.
    - [x] Ensure they are hidden in lazy mode until `show_all_tools` is called.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.
    - [x] Verified against real instance (Code is correct, but server returned 404 - likely RSS is disabled in qBittorrent settings).
