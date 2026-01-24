# :cloud: qBittorrent MCP Server (Rust)

[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/qbittorrent-mcp-rs/ci.yml?label=ci&style=for-the-badge&branch=main)](https://github.com/nicholaswilde/qbittorrent-mcp-rs/actions/workflows/ci.yml)

> [!WARNING]
> This project is currently in active development (v0.x.x) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

A Model Context Protocol (MCP) server for qBittorrent, written in Rust. This tool empowers AI agents to interact with and manage your qBittorrent instance using natural language.

## :sparkles: Features

- **:search: Search**: Search for torrents using qBittorrent's built-in search engine plugins.
- **:files: Torrent Management**:
    - List all torrents with status, progress, and speed.
    - Add torrents via Magnet URIs or HTTP URLs.
    - Pause, resume, and delete torrents.
- **:mag: Torrent Inspection**:
    - List files contained within a torrent.
    - Retrieve detailed properties (save path, creation date, seeds/peers, etc.).
- **:traffic_light: Global Control**:
    - Monitor global download/upload speeds and limits.
    - Set global download and upload speed limits.
- **:label: Categories & Tags**:
    - Organize torrents using categories and tags.
    - Create categories with dedicated save paths.
- **:sleeping: Lazy Mode**: Reduce token usage by hiding advanced tools until explicitly requested.
- **:gear: Configuration**: TOML, YAML, JSON, or CLI arguments.
- **:rocket: Transports**: Stdio (default) and HTTP (SSE).

## :downloader: Installation

### From Source

```bash
git clone https://github.com/nicholaswilde/qbittorrent-mcp-rs.git
cd qbittorrent-mcp-rs
cargo build --release
```

The binary will be at `target/release/qbittorrent-mcp-rs`.

## :wrench: Configuration

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

## :computer: Usage

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

## :hammer_and_wrench: Available Tools

The server exposes the following tools to the LLM, categorized by functionality:

### :search: Search
- `search_torrents`: Search for torrents using qBittorrent's search engine (waits 5 seconds for results).
- `install_search_plugin`: Install a new search plugin (URL).
- `uninstall_search_plugin`: Uninstall a search plugin (Name).
- `enable_search_plugin`: Enable/Disable a search plugin.
- `update_search_plugins`: Update all search plugins.
- `get_search_plugins`: List installed search plugins.

### :files: Torrent Management
- `list_torrents`: List all torrents with their status and progress.
- `add_torrent`: Add a new torrent via Magnet URI or HTTP URL.
- `pause_torrent`: Pause one or more torrents (use `|` to separate multiple hashes).
- `resume_torrent`: Resume one or more torrents (use `|` to separate multiple hashes).
- `delete_torrent`: Delete one or more torrents, optionally deleting downloaded files.

### :mag: Torrent Inspection
- `get_torrent_files`: List all files inside a specific torrent.
- `get_torrent_properties`: Get detailed technical properties of a torrent (save path, seeds, peers, etc.).

### :traffic_light: Global Control
- `get_global_transfer_info`: Get global download/upload speeds, data usage, and limits.
- `set_global_transfer_limits`: Set global download and/or upload speed limits (in bytes per second).
- `get_app_preferences`: Retrieve all application preferences (full configuration).
- `set_app_preferences`: Update application preferences using a JSON string.

### :label: Categories & Tags
- `create_category`: Create a new category with a save path.
- `set_torrent_category`: Assign a category to one or more torrents.
- `get_categories`: List all available categories.
- `add_torrent_tags`: Add tags to one or more torrents.

### :rss: RSS Management
- `add_rss_feed`: Add a new RSS feed.
- `get_rss_feeds`: List all RSS feeds and their items.
- `set_rss_rule`: Create or update an RSS auto-download rule.
- `get_rss_rules`: List all RSS auto-download rules.

### :toolbox: Utility Tools
- `wait_for_torrent_status`: Poll a torrent until it reaches a desired state (e.g., "uploading") or timeout. Useful for sequential automation without constant polling from the agent.

### :scroll: System Logs
- `get_main_log`: Retrieve the main application log (filter by severity).
- `get_peer_log`: Retrieve the peer connection log.

### :desktop: System Tools
- `show_all_tools`: Enable all available tools when running in `--lazy` mode.

## :handshake: Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/qbittorrent-mcp-rs/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[2]: <https://github.com/nicholaswilde/>
