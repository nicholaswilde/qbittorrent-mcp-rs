# qBittorrent MCP Server (Rust)

A Model Context Protocol (MCP) server for qBittorrent, written in Rust.

## Features

- **Manage Torrents**: List, add, pause, resume, delete torrents (Core foundation: List only implemented currently).
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
qbittorrent_host = "localhost"
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
- `--qbittorrent-host <host>`: qBittorrent Web UI host.
- `--qbittorrent-port <port>`: qBittorrent Web UI port.
- `--qbittorrent-username <user>`: Username.
- `--qbittorrent-password <pass>`: Password.
- `--server-mode <mode>`: `stdio` or `http`.

## Usage

### Stdio Mode (Default)

Use with an MCP client (e.g. Claude Desktop, or another MCP-compliant tool).

```bash
./qbittorrent-mcp-rs
```

### HTTP Mode

```bash
./qbittorrent-mcp-rs --server-mode http
```

Server will listen on port 3000 (default) or as configured (TODO: configure MCP server port).
SSE endpoint: `http://localhost:3000/sse`

## License

MIT
