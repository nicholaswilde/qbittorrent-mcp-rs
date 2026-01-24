# qBittorrent MCP Server (Rust)

A Model Context Protocol (MCP) server for qBittorrent, written in Rust. This tool empowers AI agents to interact with and manage your qBittorrent instance using natural language.

## Features

- **Search**: Search for torrents using qBittorrent's built-in search engine plugins.
- **Torrent Management**:
    - List all torrents with status, progress, and speed.
    - Add torrents via Magnet URIs or HTTP URLs.
    - Pause, resume, and delete torrents.
- **Torrent Inspection**:
    - List files contained within a torrent.
    - Retrieve detailed properties (save path, creation date, seeds/peers, etc.).
- **Global Control**:
    - Monitor global download/upload speeds and limits.
    - Set global download and upload speed limits.
- **Categories & Tags**:
    - Organize torrents using categories and tags.
    - Create categories with dedicated save paths.
- **Lazy Mode**: Reduce token usage by hiding advanced tools until explicitly requested.
- **Configuration**: TOML, YAML, JSON, or CLI arguments.
- **Transports**: Stdio (default) and HTTP (SSE).

## Installation

### From Source

```bash
git clone https://github.com/nicholaswilde/qbittorrent-mcp-rs.git
cd qbittorrent-mcp-rs
cargo build --release
```

The binary will be at `target/release/qbittorrent-mcp-rs`.

## Configuration

Configuration is loaded from `config.{toml,yaml,json}` in the current directory, or via CLI arguments.

### Example `config.toml`

```toml
qbittorrent_host = "localhost" # or https://your-instance.com
qbittorrent_port = 8080
qbittorrent_username = "admin"
qbittorrent_password = "password"
server_mode = "stdio" # or "http"
lazy_mode = false # or true to hide complex tools initially
```

### CLI Arguments

CLI arguments override configuration file settings.

```bash
./qbittorrent-mcp-rs --help
```

- `--config <path>`: Path to configuration file.
- `--qbittorrent-host <host>`: qBittorrent Web UI host (supports `http://` and `https://`).
- `--qbittorrent-port <port>`: qBittorrent Web UI port.
- `--qbittorrent-username <user>`: Username.
- `--qbittorrent-password <pass>`: Password.
- `--server-mode <mode>`: `stdio` or `http`.
- `--lazy`: Enable lazy mode (shows only essential tools initially to save tokens).

## Usage

### Stdio Mode (Default)

Use with an MCP client (e.g., Claude Desktop, Zed, or another MCP-compliant tool).

Add this to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "qbittorrent": {
      "command": "/path/to/qbittorrent-mcp-rs",
      "args": [
        "--qbittorrent-host", "localhost",
        "--qbittorrent-port", "8080",
        "--qbittorrent-username", "admin",
        "--qbittorrent-password", "adminadmin"
      ]
    }
  }
}
```

### HTTP Mode

```bash
./qbittorrent-mcp-rs --server-mode http
```

Server will listen on port 3000.
- SSE Endpoint: `http://localhost:3000/sse`
- Message Endpoint: `http://localhost:3000/message`

## Available Tools

The server exposes the following tools to the LLM, categorized by functionality:

### Search
- `search_torrents`: Search for torrents using qBittorrent's search engine (waits 5 seconds for results).
- `install_search_plugin`: Install a new search plugin (URL).
- `uninstall_search_plugin`: Uninstall a search plugin (Name).
- `enable_search_plugin`: Enable/Disable a search plugin.
- `update_search_plugins`: Update all search plugins.
- `get_search_plugins`: List installed search plugins.

### Torrent Management
- `list_torrents`: List all torrents with their status and progress.
- `add_torrent`: Add a new torrent via Magnet URI or HTTP URL.
- `pause_torrent`: Pause one or more torrents (use `|` to separate multiple hashes).
- `resume_torrent`: Resume one or more torrents (use `|` to separate multiple hashes).
- `delete_torrent`: Delete one or more torrents, optionally deleting downloaded files.

### Torrent Inspection
- `get_torrent_files`: List all files inside a specific torrent.
- `get_torrent_properties`: Get detailed technical properties of a torrent (save path, seeds, peers, etc.).

### Global Control
- `get_global_transfer_info`: Get global download/upload speeds, data usage, and limits.
- `set_global_transfer_limits`: Set global download and/or upload speed limits (in bytes per second).

### Categories & Tags
- `create_category`: Create a new category with a save path.
- `set_torrent_category`: Assign a category to one or more torrents.
- `get_categories`: List all available categories.
- `add_torrent_tags`: Add tags to one or more torrents.

### System Tools
- `show_all_tools`: Enable all available tools when running in `--lazy` mode.

## License

MIT