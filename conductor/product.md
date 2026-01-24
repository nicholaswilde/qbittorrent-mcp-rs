# Initial Concept

## Project Name
qBittorrent MCP Server (Rust)

## Vision
To build a robust and efficient Model Context Protocol (MCP) server using Rust that empowers AI assistants to interact with and manage qBittorrent instances seamlessly. This tool will bridge the gap between LLMs and torrent management, enabling natural language control over downloads.

## Core Features
- **API Compliance:** Adhere strictly to the [qBittorrent Web UI API v2.8.3](https://github.com/qbittorrent/qBittorrent/wiki/WebUI-API-(qBittorrent-4.1)#api-v283).
- **Torrent Management:** Add, pause, resume, and delete torrents.
- **Information Retrieval:** Query active downloads, upload/download speeds, and completion status.
- **Configuration:** Support for `config.toml`, `config.yaml`, or `config.json` files to configure the server. All configuration options can also be passed via command-line arguments, which will override file-based settings.
- **Dual Server Mode:** Support for both stdio (CLI) and HTTP server modes.
- **Safety:** Read-only mode options and secure API handling.

## Target Audience
- Developers and power users who want to automate torrent management using AI agents.
- Home lab enthusiasts integrating qBittorrent into larger AI-driven automation workflows.
