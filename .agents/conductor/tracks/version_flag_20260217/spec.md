# Specification: Version Flag Support

Add standard `--version` and `-V` command-line arguments to the `qbittorrent-mcp-rs` binary to allow users and scripts to easily identify the installed version.

## Overview
Currently, the binary does not respond to version queries. This track will implement standard version reporting flags using the existing `clap` configuration.

## Functional Requirements
- **Flag Implementation:** Support both `--version` and the short `-V` flag.
- **Output Format:** The output must follow the format `qbittorrent-mcp-rs <VERSION>`, where `<VERSION>` is pulled dynamically from `Cargo.toml` using `env!("CARGO_PKG_VERSION")`.
- **Execution Flow:** Upon detecting the version flag, the application must print the information to `stdout` and terminate immediately with exit code `0`, bypassing configuration loading and server startup.

## Non-Functional Requirements
- **Consistency:** Use `clap`'s built-in versioning support to ensure behavior matches standard Rust CLI tools.

## Acceptance Criteria
- Running `target/debug/qbittorrent-mcp-rs --version` prints `qbittorrent-mcp-rs 0.3.5` (or current version).
- Running `target/debug/qbittorrent-mcp-rs -V` prints the same.
- The application exits with code `0` after printing the version.
- No MCP server or logging infrastructure is initialized when these flags are used.

## Out of Scope
- Adding build timestamps or git commit hashes to the version string.
- Changing the versioning scheme of the project.
