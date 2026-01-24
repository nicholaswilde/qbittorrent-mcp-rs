# Advanced Functionality Implementation Plan

## Phase 1: Client Implementation
- [ ] Task: Implement Peer Management
    - [ ] `ban_peers(peers)`
- [ ] Task: Implement Advanced Torrent Control
    - [ ] `toggle_sequential_download(hashes)`
    - [ ] `toggle_first_last_piece_prio(hashes)`
    - [ ] `set_force_start(hashes, value)`
    - [ ] `set_super_seeding(hashes, value)`
- [ ] Task: Implement Tracker Management
    - [ ] `add_trackers(hashes, urls)`
    - [ ] `edit_tracker(hash, orig_url, new_url)`
    - [ ] `remove_trackers(hashes, urls)`
- [ ] Task: Implement File Management
    - [ ] `rename_file(hash, old_path, new_path)`
    - [ ] `rename_folder(hash, old_path, new_path)`
    - [ ] `set_file_priority(hash, id, priority)`
- [ ] Task: Implement Tag/Category Management
    - [ ] `remove_categories(categories)`
    - [ ] `remove_tags(hashes, tags)`
    - [ ] `create_tags(tags)`
    - [ ] `delete_tags(tags)`
- [ ] Task: Implement RSS Management
    - [ ] `move_rss_item(item_path, dest_path)`

## Phase 2: MCP Tool Integration
- [ ] Task: Register Tools in `src/server/mcp.rs`
    - [ ] `ban_peers`
    - [ ] `toggle_sequential_download`
    - [ ] `toggle_first_last_piece_prio`
    - [ ] `set_force_start`
    - [ ] `set_super_seeding`
    - [ ] `add_trackers`
    - [ ] `edit_tracker`
    - [ ] `remove_trackers`
    - [ ] `rename_file`
    - [ ] `rename_folder`
    - [ ] `set_file_priority`
    - [ ] `remove_categories`
    - [ ] `remove_tags`
    - [ ] `create_tags`
    - [ ] `delete_tags`
    - [ ] `remove_rss_item` (Already in client, just expose)
    - [ ] `move_rss_item`

## Phase 3: Verification
- [ ] Task: Unit Tests
    - [ ] Update `tests/client_management_test.rs` and others with new mocks.
- [ ] Task: CI/CD
    - [ ] Run `task test:ci`.
