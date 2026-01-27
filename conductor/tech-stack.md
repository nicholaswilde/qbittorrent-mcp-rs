# Technology Stack

## Core Language & Runtime
- **Rust:** The primary programming language for its performance and memory safety.
- **Tokio:** The asynchronous runtime for handling concurrent MCP requests and network I/O.

## MCP Server Implementation
- **mcp-rs (or official MCP SDK):** Utilizing the Model Context Protocol SDK for Rust to handle protocol negotiation and message passing.

## Communication & API
- **JSON-RPC:** The underlying protocol for MCP communication.
- **qBittorrent Web UI API (v2.8.3):** Targeted API version for all qBittorrent interactions.
- **reqwest:** For making asynchronous HTTP requests to the qBittorrent Web UI API.
- **axum:** For the HTTP server implementation.

## Configuration
- **config:** For loading configuration from `config.toml`, `config.yaml`, or `config.json` files.

## Testing & Quality Assurance
- **Cargo Test:** For unit and integration testing.
- **Clippy:** For linting and ensuring idiomatic Rust code.
- **Rustfmt:** For consistent code formatting.
- **Integration Tests:** MUST use ephemeral Docker containers.
    * **Tool:** Use the `testcontainers` library (for Rust/Java/Go) or `docker-py` (for Python).
    * **Image Source:** Default to `linuxserver/qbittorrent` for testing qBittorrent integration.
    * **Pattern:**
        1.  Spin up container on a random port.
        2.  Wait for WebUI health check (HTTP 200).
        3.  Run MCP tool against container.
        4.  Destroy container.

## Integration Testing Standard
- **Test Coverage:** All new MCP tools must include an integration test in tests/.
- **Test Templates:** Refer to tests/integration_test_template.rs for the required boilerplate.
- **Docker Integration:** Tests must use testcontainers to spin up linuxserver/qbittorrent.
- **Dynamic Networking:** Tests must dynamically fetch the host port using container.get_host_port_ipv4(8080).
