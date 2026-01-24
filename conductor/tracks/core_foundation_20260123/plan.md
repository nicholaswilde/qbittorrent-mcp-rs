# Implementation Plan - Core Foundation

## Phase 1: Project Skeleton & Configuration

- [ ] Task: Initialize Rust project and dependencies
    - [ ] Initialize new cargo project
    - [ ] Add dependencies (`tokio`, `reqwest`, `serde`, `serde_json`, `anyhow`, `tracing`, `clap`) to `Cargo.toml`
    - [ ] Set up basic logging (tracing-subscriber)
- [ ] Task: Implement Configuration Loading
    - [ ] Write Tests: Create `tests/config_test.rs` to verify env var loading
    - [ ] Implement `Config` struct and loading logic (from env vars: `QBITTORRENT_URL`, `QBITTORRENT_USERNAME`, `QBITTORRENT_PASSWORD`)
    - [ ] Verify Tests Pass
- [ ] Task: Conductor - User Manual Verification 'Project Skeleton & Configuration' (Protocol in workflow.md)

## Phase 2: qBittorrent Client (Internal API)

- [ ] Task: Define Torrent Data Models
    - [ ] Create `src/models.rs`
    - [ ] Define `Torrent` struct with `serde` derivation
    - [ ] Write unit tests for JSON deserialization (using sample qBittorrent API response)
- [ ] Task: Implement Authentication Flow
    - [ ] Write Tests: Mock `reqwest` response to test login handling
    - [ ] Implement `QBitClient::new` and `login` method
    - [ ] Verify Tests Pass
- [ ] Task: Implement `get_torrent_list`
    - [ ] Write Tests: Mock `reqwest` response for `/api/v2/torrents/info`
    - [ ] Implement `get_torrent_list` method in `QBitClient`
    - [ ] Verify Tests Pass
- [ ] Task: Conductor - User Manual Verification 'qBittorrent Client (Internal API)' (Protocol in workflow.md)

## Phase 3: MCP Server Integration

- [ ] Task: Set up MCP Server Structure
    - [ ] Add `mcp_sdk_rs` (or equivalent) dependency
    - [ ] Create basic server struct and main loop
- [ ] Task: Register `list_torrents` Tool
    - [ ] Implement the `Tool` trait/interface for `list_torrents`
    - [ ] Connect the tool execution to `QBitClient::get_torrent_list`
    - [ ] Write integration test: Simulate MCP request for tool execution
- [ ] Task: Conductor - User Manual Verification 'MCP Server Integration' (Protocol in workflow.md)
