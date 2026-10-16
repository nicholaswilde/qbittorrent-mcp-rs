# Comprehensive Testing Specification

## Goal
Verify that all implemented MCP tools function correctly against a live qBittorrent v5.x instance (`https://qbittorrent.l.nicholaswilde.io`).

## Scope
- Test all tools listed in `README.md`.
- Verify behavior with the specific version of qBittorrent.
- Fix any issues encountered during testing.

## Test Environment
- **Host**: `https://qbittorrent.l.nicholaswilde.io`
- **Method**: Using the compiled binary `qbittorrent-mcp-rs` in `stdio` mode (simulated via shell commands) or `tests/manual_real_test.rs` adapted for full coverage.

## Tools to Test
1.  **Search**: `search_torrents`, `install_search_plugin`, `uninstall_search_plugin`, `enable_search_plugin`, `update_search_plugins`, `get_search_plugins`.
2.  **Torrent Management**: `list_torrents`, `add_torrent`, `pause_torrent`, `resume_torrent`, `delete_torrent`, `reannounce_torrent`, `recheck_torrent`, `cleanup_completed`, `mass_rename`, `find_duplicates`.
3.  **Torrent Inspection**: `get_torrent_files`, `get_torrent_properties`.
4.  **Global Control**: `get_global_transfer_info`, `set_global_transfer_limits`, `toggle_alternative_speed_limits`, `get_speed_limits_mode`, `ban_peers`, `get_app_preferences`, `set_app_preferences`.
5.  **Categories & Tags**: `create_category`, `set_torrent_category`, `get_categories`, `add_torrent_tags`.
6.  **RSS**: `add_rss_feed`, `get_rss_feeds`, `set_rss_rule`, `get_rss_rules`.
7.  **Utility**: `wait_for_torrent_status`.
8.  **System**: `get_main_log`, `get_peer_log`, `get_app_version`, `get_build_info`, `shutdown_app` (Skip shutdown for shared instance safety), `show_all_tools`.
9.  **Resources**: `qbittorrent://{instance}/torrents`, etc.

## Success Criteria
- All read-only tools return valid JSON data.
- State-changing tools perform the action (verified by subsequent read).
- Any API version mismatches are resolved.
