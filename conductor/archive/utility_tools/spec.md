# Utility Tools Specification

## Goal
Implement helper tools that simplify the AI agent's interaction with qBittorrent, specifically around long-running tasks.

## Features

### 1. Wait for Torrent Status
- **Functionality**: Poll a specific torrent until it reaches a desired state or a timeout occurs.
- **Parameters**:
    - `hash`: The torrent hash.
    - `target_status`: The status to wait for (e.g., "uploading", "downloading", "stalledUP").
    - `timeout_seconds`: Maximum time to wait (default 60s, max 300s).
- **Internal Logic**: Uses `/api/v2/torrents/info?hashes=<hash>` to poll every 2-5 seconds.

## MCP Tools
- `wait_for_torrent_status(hash: String, target_status: String, timeout_seconds: Option<i64>)`
