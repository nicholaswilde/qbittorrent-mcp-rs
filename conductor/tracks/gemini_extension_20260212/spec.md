# Specification: Gemini CLI Extension for qBittorrent MCP

## Overview
This track involves transforming the `qbittorrent-mcp-rs` server into a native Gemini CLI extension. This will allow users to install, configure, and interact with their qBittorrent instances directly through the `gemini` command-line tool, providing a more integrated experience for AI-driven torrent management.

## Functional Requirements
- **Extension Manifest:** Create the required manifest file (e.g., `manifest.json` or `extension.toml`) defining the extension name, version, and entry points.
- **Binary Management:** Implement logic to automatically download or bundle the appropriate `qbittorrent-mcp-rs` binary for the user's operating system and architecture during installation.
- **Configuration Workflow:** Provide a `setup` or configuration command within the Gemini CLI to collect and store qBittorrent connection details (host, port, credentials).
- **Command Mapping:** Map the existing MCP tools to subcommands under the `qbittorrent` namespace (e.g., `gemini qbittorrent add`, `gemini qbittorrent search`).
- **MCP Integration:** Ensure the extension correctly initializes the MCP server in `stdio` mode when invoked by the Gemini CLI.

## Non-Functional Requirements
- **Cross-Platform Compatibility:** The installation and binary management must work on Linux, macOS, and Windows.
- **Zero-Dependency Installation:** Users should not need to manually install Rust or compile the binary.
- **Security:** Ensure qBittorrent credentials are handled securely according to Gemini CLI extension best practices (e.g., using `sensitive: true`).
- **Configuration Best Practices:**
    - Provide clear descriptions for each setting to guide the user.
    - Use concise, user-friendly names for settings.
    - Leverage workspace-scoped settings for project-specific configurations to avoid global pollution.

## Acceptance Criteria
- [ ] The extension can be successfully installed via the Gemini CLI.
- [ ] Running `gemini qbittorrent setup` prompts for and saves connection details.
- [ ] Running `gemini qbittorrent list` correctly lists torrents from the configured qBittorrent instance.
- [ ] All major tool categories (Search, Management, Inspection, RSS) are accessible via the CLI.
- [ ] The extension automatically handles the underlying MCP server binary without manual user intervention.

## Out of Scope
- Direct Web UI port forwarding or hosting.
- Managing multiple qBittorrent instances through a single extension install (limit to one primary instance initially).
