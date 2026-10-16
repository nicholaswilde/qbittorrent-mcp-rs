# System Log Inspection Specification

## Goal
Enable the MCP server to retrieve qBittorrent system and peer logs, allowing AI agents to diagnose issues and monitor internal events.

## Features

### 1. Get Main Log
- **Functionality**: Retrieve the main application log.
- **API Endpoint**: `/api/v2/log/main`
- **Parameters**:
    - `normal`: Include normal messages (bool).
    - `info`: Include info messages (bool).
    - `warning`: Include warning messages (bool).
    - `critical`: Include critical messages (bool).
    - `last_id`: Exclude messages with ID less than or equal to this (int).

### 2. Get Peer Log
- **Functionality**: Retrieve the peer connection log.
- **API Endpoint**: `/api/v2/log/peers`
- **Parameters**:
    - `last_id`: Exclude messages with ID less than or equal to this (int).

## MCP Tools
- `get_main_log(severity: Option<String>, last_id: Option<i64>)`:
    - `severity` can be "info", "warning", "critical", or "all".
- `get_peer_log(last_id: Option<i64>)`: Retrieve peer logs.
