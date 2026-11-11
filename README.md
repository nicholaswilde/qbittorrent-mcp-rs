# :magnet: qBittorrent MCP Server (Rust) :robot:
[![Coveralls](https://img.shields.io/coveralls/github/nicholaswilde/qbittorrent-mcp-rs/main?style=for-the-badge&logo=coveralls)](https://coveralls.io/github/nicholaswilde/qbittorrent-mcp-rs?branch=main)
[![task](https://img.shields.io/badge/Task-Enabled-brightgreen?style=for-the-badge&logo=task&logoColor=white)](https://taskfile.dev/#/)
[![ci](https://img.shields.io/github/actions/workflow/status/nicholaswilde/qbittorrent-mcp-rs/ci.yml?label=ci&style=for-the-badge&branch=main&logo=github-actions)](https://github.com/nicholaswilde/qbittorrent-mcp-rs/actions/workflows/ci.yml)

> [!WARNING]
> This project is currently in active development (v0.3.9) and is **not production-ready**. Features may change, and breaking changes may occur without notice.

A [Model Context Protocol (MCP) server](https://modelcontextprotocol.io/docs/getting-started/intro) for [qBittorrent](https://www.qbittorrent.org/), written in Rust. This tool empowers AI agents to interact with and manage your qBittorrent instance using natural language.

## :sparkles: Features

- **:globe_with_meridians: Search**: Search for torrents using qBittorrent's built-in search engine plugins.
- **:card_file_box: Torrent Management**:
    - Manage multiple qBittorrent instances simultaneously.
    - List all torrents with status, progress, and speed.
    - Add torrents via Magnet URIs or HTTP URLs.
    - **Unified Control**: Perform actions like pause, resume, reannounce, set category, and add tags via a single `manage_torrents` tool.
- **:mag: Torrent Inspection**:
    - **Consolidated Inspection**: Retrieve properties, files, and trackers in a single call via `inspect_torrent`.
    - Deep file listing and tracker status monitoring.
- **:traffic_light: Global Control**:
    - **System Health**: Monitor global speeds, data usage, and limits via a unified `get_system_info` tool.
    - Ban peers and update application preferences.
- **:label: Categories & Tags**: Organize torrents using categories and tags with simplified management tools.
- **:mega: Proactive Notifications**: Receive real-time notifications when downloads finish (powered by the Sync API).
- **:broom: Maintenance Macros**:
    - `cleanup_completed`: Auto-remove torrents based on seeding ratio or age.
    - `mass_rename`: Bulk rename files within torrents using Regex.
    - `find_duplicates`: Identify redundant downloads by name.
- **:bulb: Troubleshooting Prompts**: Consolidated `troubleshoot_torrent` prompt for guided workflows.
- **:sleeping: Lazy Mode**: Reduce token usage by hiding advanced tools until explicitly requested.
- **:gear: Configuration**: TOML, YAML, JSON, or Environment Variables.
- **:rocket: Transports**: Stdio (default) and HTTP (SSE and Streamable HTTP, bearer-token auth required).
- **:books: Resources**: Scoped resources for all instances (e.g., `qbittorrent://seedbox/torrents`).

## :hammer_and_wrench: Available Tools

The server exposes the following tools to the LLM, categorized by functionality:

### :globe_with_meridians: Search
- `search_torrents`: Search for torrents using qBittorrent's search engine. Waits up to `wait_seconds` (default 5), returns at most `limit` results (default 25), then removes the search job.
- `install_search_plugin`: Install a new search plugin (URL).
- `uninstall_search_plugin`: Uninstall a search plugin (Name).
- `enable_search_plugin`: Enable/Disable a search plugin.
- `update_search_plugins`: Update all search plugins.
- `get_search_plugins`: List installed search plugins.

### :card_file_box: Torrent Management
- `list_torrents`: List torrents with their status and progress. Supports filtering, sorting, and optional inclusion of `properties` or `files`. Listings return 50 torrents unless `limit` is set; `stats_only` and `group_by_*` always cover all torrents.
- `manage_torrents`: Unified tool for multiple torrent actions: `pause`, `resume`, `reannounce`, `recheck`, `set_category`, `add_tags`, `remove_tags`, `set_share_limits`, `set_speed_limits`, `toggle_sequential`, `toggle_first_last_prio`, `set_force_start`, `set_super_seeding`, `set_location` (move data), `rename`, `set_auto_management`, `top_prio`, `bottom_prio`, `increase_prio`, `decrease_prio`, `add_peers`.
- `add_torrent`: Add torrents via Magnet URI / HTTP URL or a base64 `.torrent` file, with tags, rename, paused, skip checking, sequential, limits and more.
- `delete_torrent`: Delete one or more torrents, optionally deleting downloaded files.
- `cleanup_completed`: Remove completed torrents by minimum ratio and/or age (days). Dry run by default: lists matches until called with `dry_run: false`.
- `mass_rename`: Rename multiple files in a torrent using a Regex pattern and replacement string.
- `find_duplicates`: Group and list torrents with identical names.
- `add_trackers`: Add trackers to torrents.
- `edit_tracker`: Edit a tracker URL for a torrent.
- `remove_trackers`: Remove trackers from torrents.
- `rename_folder`: Rename a folder in a torrent.
- `set_file_priority`: Set priority for files in a torrent.

### :mag: Torrent Inspection
- `inspect_torrent`: Comprehensive inspection of a torrent (properties, files, and trackers in one call; optional peers, web seeds, piece states and piece hashes).
- `get_torrent_files`: List all files inside a specific torrent.

### :traffic_light: Global Control
- `ban_peers`: Ban a list of peers (host:port, pipe-separated).
- `set_global_speed_limits`: Set global download/upload limits and turn alternative speed limits on or off.
- `set_app_preferences`: Update application preferences using a JSON string. Keys that run programs (`autorun_*`), control Web UI access (`web_ui_*`, `bypass_*`) or hold passwords are refused.
- `manage_cookies`: Get or set cookies used when downloading `.torrent` files from URLs.

### :label: Categories & Tags
- `get_categories`: List all available categories.
- `manage_categories`: Create, edit (save path) or remove categories.
- `manage_tags`: List, create or delete tags.

### :wireless: RSS Management
- `add_rss_feed`: Add a new RSS feed.
- `get_rss_feeds`: List all RSS feeds and their items.
- `set_rss_rule`: Create or update an RSS auto-download rule.
- `get_rss_rules`: List all RSS auto-download rules.
- `move_rss_item`: Move an RSS item (feed or folder).
- `manage_rss`: Remove a feed/folder, add a folder, mark as read, refresh, remove or rename a rule, list a rule's matching articles.

### :toolbox: Utility Tools
- `wait_for_torrent_status`: Poll a torrent until it reaches a desired state (e.g., "uploading") or timeout. Useful for sequential automation without constant polling from the agent.

### :scroll: System Logs
- `get_main_log`: Retrieve the main application log (filter by severity).
- `get_peer_log`: Retrieve the peer connection log.

### :desktop_computer: System Tools
- `get_system_info`: Comprehensive system information (transfer speeds and limits, alternative speed mode, default save path, preferences with secrets removed, app and Web API version, build info).
- `shutdown_app`: Shutdown the qBittorrent application.
- `show_all_tools`: Enable all available tools when running in `--lazy` mode.

## :books: Resources

The server exposes the following resources, scoped by instance name:

- `qbittorrent://{instance}/torrents`: Live list of all torrents (JSON).
- `qbittorrent://{instance}/transfer`: Global transfer statistics and limits (JSON).
- `qbittorrent://{instance}/categories`: List of all defined categories (JSON).

And templates for deep inspection:
- `qbittorrent://{instance}/torrent/{hash}/properties`: Comprehensive metadata.
- `qbittorrent://{instance}/torrent/{hash}/files`: File structure and individual progress.
- `qbittorrent://{instance}/torrent/{hash}/trackers`: Tracker status and messages.

## :gear: Installation

### Homebrew

```bash
brew tap nicholaswilde/tap
brew install qbittorrent-mcp-rs
```

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
# qbittorrent_port = 8080      # Optional. Defaults to 80 for http, 443 for https.
qbittorrent_username = "admin"
qbittorrent_password = "password"
server_mode = "stdio"          # or "http"
lazy_mode = false              # or true to hide complex tools initially
no_verify_ssl = false          # or true to disable SSL verification
polling_interval_ms = 2000     # Optional. Interval for proactive notifications (default: 2000ms)
# http_auth_token = "secret"   # Required in HTTP mode
# http_host = "0.0.0.0"        # HTTP listen address (default 0.0.0.0)
# http_port = 3000             # HTTP listen port (default 3000)
```

### Environment Variables

Environment variables override configuration file settings. Use `_` (single underscore) after the `QBITTORRENT` prefix, and `__` (double underscore) as a separator for nested fields.

- `QBITTORRENT_SERVER_MODE`: `stdio` or `http`
- `QBITTORRENT_LAZY_MODE`: `true` or `false`
- `QBITTORRENT_HTTP_AUTH_TOKEN`: Token for HTTP mode (required in HTTP mode).
- `QBITTORRENT_HTTP_HOST`: HTTP listen address (default `0.0.0.0`).
- `QBITTORRENT_HTTP_PORT`: HTTP listen port (default `3000`).
- `QBITTORRENT_LOG_LEVEL`: `error`, `warn`, `info`, `debug`, `trace`.
- `QBITTORRENT_POLLING_INTERVAL_MS`: Polling interval in milliseconds.

**Single Instance:**
- `QBITTORRENT_HOST`: Host address.
- `QBITTORRENT_PORT`: Port number.
- `QBITTORRENT_USERNAME`: Username.
- `QBITTORRENT_PASSWORD`: Password.

**Multiple Instances:**
Use the `QBITTORRENT_INSTANCES__<index>__<field>` pattern:
- `QBITTORRENT_INSTANCES__0__NAME=local`
- `QBITTORRENT_INSTANCES__0__HOST=localhost`
- `QBITTORRENT_INSTANCES__1__NAME=seedbox`
- `QBITTORRENT_INSTANCES__1__HOST=seedbox.example.com`

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
- `--no-verify-ssl`: Disable SSL certificate verification (insecure).
- `--http-auth-token <token>`: Authentication token for HTTP mode (required in HTTP mode).
- `--http-host <addr>`: HTTP listen address (default `0.0.0.0`).
- `--http-port <port>`: HTTP listen port (default `3000`).
- `--polling-interval-ms <ms>`: Polling interval for notifications (ms).
- `--log-level <level>`: Log level (error, warn, info, debug, trace).
- `--log-file-enable`: Enable logging to a file.
- `--log-dir <dir>`: Log file directory.
- `--log-filename <name>`: Log filename prefix.
- `--log-rotate <strategy>`: Log rotation strategy (daily, hourly, never).

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
./qbittorrent-mcp-rs --server-mode http --http-auth-token "$(openssl rand -hex 24)" --http-port 3000
```

HTTP mode refuses to start without a token. Clients must send `Authorization: Bearer <token>`; tokens in the URL are not accepted.

- Streamable HTTP endpoint: `http://localhost:3000/mcp` (POST, JSON response)
- SSE endpoint (2024-11-05 transport): `http://localhost:3000/sse`, with messages posted to `/message`. Finished-download notifications are pushed to SSE sessions.

Example for a client that supports remote servers:

```json
{
  "mcpServers": {
    "qbittorrent": {
      "url": "http://nas.local:3000/mcp",
      "headers": { "Authorization": "Bearer <token>" }
    }
  }
}
```

The Docker image runs as a non-root user (UID 65532). If you enable file logging into a mounted volume, make that folder writable for that user.

## :hammer_and_wrench: Development

### Code Coverage

To generate local code coverage reports, ensure you have `cargo-llvm-cov` installed:

```bash
cargo install cargo-llvm-cov
```

Run the following command to execute tests and generate a console summary along with a `lcov.info` file:

```bash
task coverage
```

The `lcov.info` file is compatible with IDE extensions like **VS Code Coverage Gutters**.

#### Reporting to Coveralls.io

To upload coverage data to [Coveralls.io](https://coveralls.io/), set your repository token and run the upload task:

```bash
export COVERALLS_REPO_TOKEN=your_token_here
task coverage:upload
```

#### Release Management

This project includes a `gemini-cli` command to help generate GitHub release summaries based on your local git log.

```bash
# Summarize changes between the latest two tags (automatic)
gemini release:summary

# Summarize changes for a specific range or tag
gemini release:summary --range v0.3.2..v0.3.3
```

### Secrets Management

The project uses [SOPS](https://github.com/getsops/sops) for encrypting sensitive files like `.env`.

- **Encrypt**: `task encrypt` (requires `.env`)
- **Decrypt**: `task decrypt` (requires `.env.enc`)

## :mag: Troubleshooting

### Connection & Authentication Issues

- **Login failed: wrong username or password**: qBittorrent rejected the credentials. The server now says so at startup instead of failing later with 403 on every call.
- **Login failed: 403 Forbidden**: qBittorrent banned this IP after too many failed logins. Restart qBittorrent or wait for the ban to expire. The server backs off polling (up to 5 minutes) after failures so it does not keep itself banned.
- **401 Unauthorized**: Ensure your username and password are correct. If running qBittorrent 4.6.1+ or 5.x, ensure `WebUI\HostHeaderValidation=false` is set in your `qBittorrent.conf` if you are accessing it via a custom hostname or reverse proxy.
- **CSRF Protection**: If you encounter 403 Forbidden errors during login, ensure `WebUI\CSRFProtection=false` is set in your configuration, or that the server's `Origin` and `Referer` headers (handled automatically by this server) match the expected host.
- **Docker Networking**: If running the MCP server outside of Docker and qBittorrent inside, use the host's IP address or `localhost` (if ports are mapped). If both are in Docker, use the container name as the host.

## :handshake: Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](.github/CONTRIBUTING.md) for guidelines.

## :balance_scale: License

​[​Apache License 2.0](https://raw.githubusercontent.com/nicholaswilde/qbittorrent-mcp-rs/refs/heads/main/LICENSE)

## :writing_hand: Author

​This project was started in 2026 by [Nicholas Wilde][2].

[2]: <https://github.com/nicholaswilde/>
