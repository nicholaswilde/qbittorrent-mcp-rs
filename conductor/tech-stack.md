# Technology Stack

## Core Language & Runtime
- **Rust:** The primary programming language for its performance and memory safety.
- **Tokio:** The asynchronous runtime for handling concurrent MCP requests and network I/O.

## MCP Server Implementation
- **mcp-rs (or official MCP SDK):** Utilizing the Model Context Protocol SDK for Rust to handle protocol negotiation and message passing.

## Communication & API
- **JSON-RPC:** The underlying protocol for MCP communication.
- **reqwest:** For making asynchronous HTTP requests to the qBittorrent Web UI API.

## Testing & Quality Assurance
- **Cargo Test:** For unit and integration testing.
- **Clippy:** For linting and ensuring idiomatic Rust code.
- **Rustfmt:** For consistent code formatting.
