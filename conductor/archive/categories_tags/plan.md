# Categories & Tags Implementation Plan

## Phase 1: Client Implementation
- [x] Task: Define Models
    - [x] Create `Category` struct in `src/models.rs`.
- [x] Task: Implement Methods in `QBitClient`
    - [x] `create_category(name, save_path)`.
    - [x] `set_category(hashes, category)`.
    - [x] `get_categories()`.
    - [x] `add_tags(hashes, tags)`.
    - [x] `create_tags(tags)`.
- [x] Task: Unit Tests
    - [x] Create `tests/client_categories_test.rs`.

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools
    - [x] `create_category`
    - [x] `set_torrent_category`
    - [x] `get_categories`
    - [x] `add_torrent_tags`
    - [x] Update `tools/list` and `tools/call`.

## Phase 3: Verification
- [x] Task: Verify Compilation and Tests
    - [x] Run `cargo test`.
    - [x] Build project.
    - [x] Verified against real instance (`create_category` and `get_categories` confirmed working).
