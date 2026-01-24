# qBittorrent MCP Server (Rust)

A Model Context Protocol (MCP) server for qBittorrent, written in Rust. This tool empowers AI agents to interact with and manage your qBittorrent instance using natural language.

## Features

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

The server exposes the following tools to the LLM:

- `list_torrents`: List all torrents.
- `add_torrent`: Add a new torrent (url, save_path, category).
- `pause_torrent`: Pause a torrent (hash).
- `resume_torrent`: Resume a torrent (hash).
- `delete_torrent`: Delete a torrent (hash, delete_files).
- `get_torrent_files`: List files inside a torrent (hash).
- `get_torrent_properties`: Get detailed properties of a torrent (hash).
- `get_global_transfer_info`: Get global speed and limits.
- `set_global_transfer_limits`: Set global download/upload limits.

## License

MIT