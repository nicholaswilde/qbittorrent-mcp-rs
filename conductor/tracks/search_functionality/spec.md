# Search Functionality Specification

## Goal
Enable the MCP server to search for torrents using qBittorrent's built-in search engine.

## Features

### 1. Search Management
- **Start Search**: Initiate a search query.
    - Endpoint: `/api/v2/search/start`
    - Returns: Search ID.
- **Get Results**: Retrieve results for a running search.
    - Endpoint: `/api/v2/search/results`
- **Stop/Delete Search**: Clean up search jobs.
    - Endpoint: `/api/v2/search/stop`, `/api/v2/search/delete`

### 2. Search Plugins (Optional/Future)
- Manage search plugins if needed, but standard enabled plugins usually suffice.

## MCP Tools
- `search_torrents(query: String, category: Option<String>)`:
    - This tool will act as a high-level wrapper.
    - **Logic**:
        1. Call `start`.
        2. Wait/Poll `results` for a fixed duration (e.g., 5-10 seconds) or until status is 'Stopped'.
        3. Call `stop` and `delete`.
        4. Return the aggregated results.
    - **Output**: JSON list of results (name, size, seeds, leechers, file_url/magnet).

## Data Models
### SearchJob
- `id`: Integer

### SearchResult
- `fileName`: String
- `fileUrl`: String (Magnet or HTTP)
- `fileSize`: Integer
- `nbSeeders`: Integer
- `nbLeechers`: Integer
- `siteUrl`: String
