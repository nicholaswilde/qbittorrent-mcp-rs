# Advanced Functionality Implementation Plan [checkpoint: 78357ff]

## Phase 1: Client Implementation
- [x] Task: Implement Peer Management
    - [x] `ban_peers(peers)`
- [x] Task: Implement Advanced Torrent Control
    - [x] `toggle_sequential_download(hashes)`
    - [x] `toggle_first_last_piece_prio(hashes)`
    - [x] `set_force_start(hashes, value)`
    - [x] `set_super_seeding(hashes, value)`
- [x] Task: Implement Tracker Management
    - [x] `add_trackers(hashes, urls)`
    - [x] `edit_tracker(hash, orig_url, new_url)`
    - [x] `remove_trackers(hashes, urls)`
- [x] Task: Implement File Management
    - [x] `rename_file(hash, old_path, new_path)`
    - [x] `rename_folder(hash, old_path, new_path)`
    - [x] `set_file_priority(hash, id, priority)`
- [x] Task: Implement Tag/Category Management
    - [x] `remove_categories(categories)`
    - [x] `remove_tags(hashes, tags)`
    - [x] `create_tags(tags)`
    - [x] `delete_tags(tags)`
- [x] Task: Implement RSS Management
    - [x] `move_rss_item(item_path, dest_path)`

## Phase 2: MCP Tool Integration
- [x] Task: Register Tools in `src/server/mcp.rs`
    - [x] `ban_peers`
    - [x] `toggle_sequential_download`
    - [x] `toggle_first_last_piece_prio`
    - [x] `set_force_start`
    - [x] `set_super_seeding`
    - [x] `add_trackers`
    - [x] `edit_tracker`
    - [x] `remove_trackers`
    - [x] `rename_file`
    - [x] `rename_folder`
    - [x] `set_file_priority`
    - [x] `remove_categories`
    - [x] `remove_tags`
    - [x] `create_tags`
    - [x] `delete_tags`
    - [x] `remove_rss_item` (Already in client, just expose)
    - [x] `move_rss_item`

## Phase 3: Verification
- [x] Task: Unit Tests
    - [x] Update `tests/client_management_test.rs` and others with new mocks.
- [x] Task: CI/CD
    - [x] Run `task test:ci`.
