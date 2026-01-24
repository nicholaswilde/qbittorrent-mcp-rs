# Specification: Core MCP Server Foundation

## Goal
To establish a functional MCP server in Rust that can connect to a qBittorrent instance and expose a single tool, `list_torrents`. The server must support both `stdio` and `http` transport modes and load configuration from `config.toml`, `config.yaml`, or `config.json`.

## Core Features
1.  **Dual Mode MCP Server:** A running process that can operate in either `stdio` mode (for CLI/Agent integration) or `http` mode (SSE/Post).
2.  **Configuration System:** Support for loading settings from `config.toml`, `config.yaml`, or `config.json`.
3.  **qBittorrent Client:** An internal Rust module that manages authentication and HTTP requests to the qBittorrent Web UI API.
4.  **Tool: `list_torrents`:** An MCP tool that, when invoked, returns a JSON list of torrents (name, state, progress) from the qBittorrent server.

## Architecture
- **Runtime:** `tokio` for async execution.
- **Protocol:** `mcp-rs` (or equivalent SDK pattern) to handle the MCP lifecycle.
- **HTTP Server:** `axum` for the HTTP transport mode.
- **HTTP Client:** `reqwest` for communicating with qBittorrent.
- **Configuration:** `config` crate for multi-format file loading.

## Data Structures
- **`Torrent`:** A Rust struct representing a torrent's key data (hash, name, size, progress, state).
- **`Config`:** A struct to hold connection details and server settings.

## Success Criteria
- The server compiles and runs without errors.
- It can start in `stdio` mode and respond to JSON-RPC.
- It can start in `http` mode and respond to health checks or SSE connections.
- It correctly loads configuration from a file (TOML/YAML/JSON).
- It successfully connects to a local or mocked qBittorrent instance.
- Invoking `list_torrents` returns valid data.