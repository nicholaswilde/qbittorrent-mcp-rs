# Comprehensive Testing Report

## Execution Details
- **Date**: 2026-01-26
- **Target**: qBittorrent v5.1.4 (WebAPI v2.11.4)
- **Host**: `https://qbittorrent.l.nicholaswilde.io`

## Summary
All MCP tools were tested against a live qBittorrent v5 instance. Several compatibility issues were identified and fixed.

## Findings & Fixes

### 1. Pause/Resume Torrent
- **Issue**: `404 Not Found` when calling `pause_torrent` / `resume_torrent`.
- **Cause**: qBittorrent v5 renamed endpoints to `/api/v2/torrents/stop` and `/api/v2/torrents/start`.
- **Fix**: Updated `QBitClient` to try v5 endpoints first, with fallback to v4 endpoints.

### 2. Search Functionality
- **Issue 1**: `error decoding response body` for `get_search_plugins`.
    - **Cause**: v5 API schema changes (fields might be missing or renamed).
    - **Fix**: Relaxed `SearchPlugin` struct fields to be `Option<T>`.
- **Issue 2**: `400 Bad Request` for `start_search`.
    - **Cause**: Missing mandatory `plugins` and `category` parameters in v5.
    - **Fix**: Updated `start_search` to default `plugins="all"` and `category="all"`.

### 3. RSS Management
- **Issue**: `404 Not Found` for `get_rss_feeds` and `get_rss_rules`.
    - **Cause**: Endpoints renamed/consolidated in v5.
        - `allFeeds` -> `items?withData=true`
        - `allRules` -> `rules`
    - **Fix**: Updated `QBitClient` to use v5 endpoints with fallback to legacy endpoints.

### 4. Torrent Limits
- **Issue**: `400 Bad Request` for `set_torrent_share_limits`.
    - **Cause**: Missing `inactiveSeedingTimeLimit` parameter required by newer API versions.
    - **Fix**: Added optional `inactive_seeding_time_limit` parameter to tool and client, defaulting to `-2` (global) if not provided.

## Verification
A full coverage test suite (`tests/full_coverage_real_test.rs`) was executed, confirming all tools now function correctly:
- ✅ Authentication
- ✅ System Info (Version, Build)
- ✅ Global Transfer Info
- ✅ Torrent Management (List, Properties, Files, Trackers, Pause, Resume, Recheck)
- ✅ Torrent Limits (Share, Speed)
- ✅ Categories (List, Create)
- ✅ Search (Plugins, Start, Stop, Delete)
- ✅ RSS (Feeds, Rules)
- ✅ Logs (Main, Peer)
- ✅ App Preferences

## Conclusion
The MCP server is now fully compatible with qBittorrent v5.x while maintaining backward compatibility where possible.
