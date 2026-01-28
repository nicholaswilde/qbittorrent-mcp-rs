# Global Transfer Control Specification

## Goal
Enable the MCP server to monitor and control global transfer settings, such as speeds and limits.

## Features

### 1. Get Global Transfer Info
- **Functionality**: Retrieve current global download/upload speeds and limits.
- **API Endpoint**: `/api/v2/transfer/info`

### 2. Set Global Speed Limits
- **Functionality**: Set global download and upload speed limits.
- **API Endpoint**: `/api/v2/transfer/setDownloadLimit` and `/api/v2/transfer/setUploadLimit`
- **Parameters**:
    - `limit`: i64 (bytes per second).

## MCP Tools
The following tools will be exposed to the LLM:
- `get_global_transfer_info()`: Returns current speeds, limits, and connection status.
- `set_global_transfer_limits(dl_limit: Option<i64>, up_limit: Option<i64>)`: Sets download and/or upload limits.
