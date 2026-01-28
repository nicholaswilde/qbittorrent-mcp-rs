# Torrent Management Specification

## Goal
Enable the MCP server to actively manage torrents on the qBittorrent instance.

## Features

### 1. Add Torrent
- **Functionality**: Add new torrents via Magnet URIs or HTTP URLs.
- **API Endpoint**: `/api/v2/torrents/add`
- **Parameters**:
    - `urls`: String (URLs separated by newlines).
    - `savepath`: Optional string.
    - `category`: Optional string.
    - `paused`: Optional boolean (default false).

### 2. Pause Torrent
- **Functionality**: Pause one or more torrents.
- **API Endpoint**: `/api/v2/torrents/pause`
- **Parameters**:
    - `hashes`: String (hashes separated by `|`).

### 3. Resume Torrent
- **Functionality**: Resume one or more torrents.
- **API Endpoint**: `/api/v2/torrents/resume`
- **Parameters**:
    - `hashes`: String (hashes separated by `|`).

### 4. Delete Torrent
- **Functionality**: Delete one or more torrents, optionally deleting files.
- **API Endpoint**: `/api/v2/torrents/delete`
- **Parameters**:
    - `hashes`: String (hashes separated by `|`).
    - `deleteFiles`: Boolean (true/false).

## MCP Tools
The following tools will be exposed to the LLM:
- `add_torrent(url: String, save_path: Option<String>, category: Option<String>)`
- `pause_torrent(hash: String)`
- `resume_torrent(hash: String)`
- `delete_torrent(hash: String, delete_files: bool)`
