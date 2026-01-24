# Specification: Core MCP Server Foundation

## Goal
To establish a functional MCP server in Rust that can connect to a qBittorrent instance and expose a single tool, `list_torrents`, which retrieves the list of active downloads. This serves as the "Hello World" and architectural skeleton for the project.

## Core Features
1.  **MCP Server Shell:** A running process that speaks the Model Context Protocol (JSON-RPC) over stdio.
2.  **qBittorrent Client:** An internal Rust module that manages authentication and HTTP requests to the qBittorrent Web UI API.
3.  **Tool: `list_torrents`:** An MCP tool that, when invoked, returns a JSON list of torrents (name, state, progress) from the qBittorrent server.

## Architecture
- **Runtime:** `tokio` for async execution.
- **Protocol:** `mcp-rs` (or equivalent SDK pattern) to handle the MCP lifecycle.
- **HTTP Client:** `reqwest` for communicating with qBittorrent.
- **Configuration:** Environment variables for qBittorrent URL, username, and password.

## Data Structures
- **`Torrent`:** A Rust struct representing a torrent's key data (hash, name, size, progress, state).
- **`Config`:** A struct to hold connection details.

## Success Criteria
- The server compiles and runs without errors.
- It successfully connects to a local or mocked qBittorrent instance.
- An MCP client (or a test script) can list the available tools and see `list_torrents`.
- Invoking `list_torrents` returns valid data.
