# Advanced Functionality Specification

## Goal
Implement missing advanced features from the qBittorrent WebUI API v2 to provide comprehensive control via MCP.

## Scope

### 1. Peer Management
- **Ban Peers**: `/api/v2/transfer/banPeers`
    - Input: `peers` (host:port, pipe-separated)

### 2. Advanced Torrent Control
- **Toggle Sequential Download**: `/api/v2/torrents/toggleSequencialDownload`
- **Toggle First/Last Piece Priority**: `/api/v2/torrents/toggleFirstLastPiecePrio`
- **Set Force Start**: `/api/v2/torrents/setForceStart`
- **Set Super Seeding**: `/api/v2/torrents/setSuperSeeding`

### 3. Tracker Management
- **Add Trackers**: `/api/v2/torrents/addTrackers`
- **Edit Tracker**: `/api/v2/torrents/editTracker`
- **Remove Trackers**: `/api/v2/torrents/removeTrackers`

### 4. File Management
- **Rename File**: `/api/v2/torrents/renameFile`
- **Rename Folder**: `/api/v2/torrents/renameFolder`
- **Set File Priority**: `/api/v2/torrents/setFilePrio`

### 5. Tag & Category Management
- **Remove Categories**: `/api/v2/torrents/removeCategories`
- **Remove Tags (from torrent)**: `/api/v2/torrents/removeTags`
- **Create Tags (global)**: `/api/v2/torrents/createTags`
- **Delete Tags (global)**: `/api/v2/torrents/deleteTags`

### 6. RSS Management
- **Remove Item**: Expose `remove_rss_item` in MCP.
- **Move Item**: `/api/v2/rss/moveItem`

## Detailed Tool Definitions

### `ban_peers`
- **Description**: Ban a list of peers.
- **Args**: `peers` (string).

### `toggle_sequential_download`
- **Description**: Toggle sequential download for torrents.
- **Args**: `hashes` (string).

### `add_trackers`
- **Description**: Add trackers to torrents.
- **Args**: `hashes` (string), `urls` (string).

### `rename_file`
- **Description**: Rename a file inside a torrent.
- **Args**: `hash` (string), `old_path` (string), `new_path` (string).

*(And so on for other endpoints)*
