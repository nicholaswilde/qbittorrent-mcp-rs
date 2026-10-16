# Implementation Plan - Core Foundation

## Phase 1: Project Skeleton & Configuration [checkpoint: a081498]

- [x] Task: Initialize Rust project and dependencies
    - [x] Initialize new cargo project
    - [x] Add dependencies (`tokio`, `reqwest`, `serde`, `serde_json`, `anyhow`, `tracing`, `clap`, `config`, `axum`, `tower-http`) to `Cargo.toml`
    - [x] Set up basic logging (tracing-subscriber)
- [x] Task: Implement Configuration Loading
    - [x] Write Tests: Create `tests/config_test.rs` to verify loading from TOML, YAML, and JSON, and verify CLI argument overrides
    - [x] Implement `Config` struct and loading logic using the `config` crate (merging default, file, env vars, and CLI arguments via `clap`)
    - [x] Verify Tests Pass
- [x] Task: Conductor - User Manual Verification 'Project Skeleton & Configuration' (Protocol in workflow.md)

## Phase 2: qBittorrent Client (Internal API v2.8.3) [checkpoint: a8531f0]

- [x] Task: Define Torrent Data Models
    - [x] Create `src/models.rs`
    - [x] Define `Torrent` struct with `serde` derivation based on API v2.8.3 specs
    - [x] Write unit tests for JSON deserialization (using sample qBittorrent API response)
- [x] Task: Implement Authentication Flow
    - [x] Write Tests: Mock `reqwest` response to test login handling (SID-based)
    - [x] Implement `QBitClient::new` and `login` method
    - [x] Verify Tests Pass
- [x] Task: Implement `get_torrent_list`
    - [x] Write Tests: Mock `reqwest` response for `/api/v2/torrents/info` following v2.8.3 schema
    - [x] Implement `get_torrent_list` method in `QBitClient`
    - [x] Verify Tests Pass
- [x] Task: Conductor - User Manual Verification 'qBittorrent Client (Internal API)' (Protocol in workflow.md)

## Phase 3: MCP Server Integration (Dual Mode) [checkpoint: edbf6a9]

- [x] Task: Set up MCP Server Structure & CLI Args
    - [x] Implement `clap` arguments to select between `stdio` and `http` modes (and specify config path)
    - [x] Create basic server traits/structs to abstract the transport layer
- [x] Task: Implement Stdio Transport
    - [x] Integrate `mcp_sdk_rs` (or equivalent) for stdio
    - [x] Write test/verification script
- [x] Task: Implement HTTP Transport
    - [x] Set up `axum` router for MCP (SSE endpoint, POST endpoint)
    - [x] Write test/verification script
- [x] Task: Register `list_torrents` Tool
    - [x] Implement the `Tool` trait/interface for `list_torrents`
    - [x] Connect the tool execution to `QBitClient::get_torrent_list`
    - [x] Verify tool availability in both transports
- [x] Task: Update README
    - [x] Create/Update `README.md` with installation, configuration (file formats), and usage instructions (stdio vs http)
- [x] Task: Conductor - User Manual Verification 'MCP Server Integration' (Protocol in workflow.md)