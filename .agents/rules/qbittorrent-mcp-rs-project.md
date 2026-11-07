# qBittorrent MCP Server Project Rules

- **Test-Driven Development (TDD)**: Strictly mandatory. Always write failing tests first (Red), implement minimum code to pass (Green), then refactor. Never write unverified implementation code without prior failing tests.
- **MCP Standards Compliance**: Adhere strictly to the Model Context Protocol specifications for stdio and HTTP/SSE transports. Tools must return structured content blocks and handle errors gracefully.
- **Multi-Architecture**: Always preserve cross-compilation compatibility via `cross` across target architectures (`amd64`, `arm64`, `armv7`, `armv6`).
- **Standard Stack Tools**:
  - Task runner: `task` (`Taskfile.yml`).
  - Secret safety: Never commit plain `.env` files or credentials. Use SOPS (`task encrypt` / `task decrypt`).
  - Profiles: Keep `[profile.release]` size-optimized (`opt-level = "z"`, `lto = true`, `strip = true`, `panic = "abort"`).
- **Token Efficiency**: Consolidated tools (`inspect_torrent`, `manage_torrents`, `get_system_info`) and optional lazy mode to conserve context tokens.
