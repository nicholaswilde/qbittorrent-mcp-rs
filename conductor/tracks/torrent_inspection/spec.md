# Torrent Inspection Specification

## Goal
Enable the MCP server to provide detailed information about specific torrents, including their file contents and advanced properties.

## Features

### 1. Get Torrent Files
- **Functionality**: List files within a torrent.
- **API Endpoint**: `/api/v2/torrents/files`
- **Parameters**:
    - `hash`: String.

### 2. Get Torrent Properties
- **Functionality**: Retrieve generic properties for a torrent.
- **API Endpoint**: `/api/v2/torrents/properties`
- **Parameters**:
    - `hash`: String.

## MCP Tools
The following tools will be exposed to the LLM:
- `get_torrent_files(hash: String)`: Returns a list of files with their indices, names, sizes, and progress.
- `get_torrent_properties(hash: String)`: Returns detailed properties like save path, creation date, seeds/peers, speed, etc.
