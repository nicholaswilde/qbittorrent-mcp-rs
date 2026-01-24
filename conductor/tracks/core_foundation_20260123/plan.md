# Implementation Plan - Core Foundation

## Phase 1: Project Skeleton & Configuration

- [ ] Task: Initialize Rust project and dependencies
    - [ ] Initialize new cargo project
    - [ ] Add dependencies (`tokio`, `reqwest`, `serde`, `serde_json`, `anyhow`, `tracing`, `clap`, `config`, `axum`, `tower-http`) to `Cargo.toml`
    - [ ] Set up basic logging (tracing-subscriber)
- [ ] Task: Implement Configuration Loading
    - [ ] Write Tests: Create `tests/config_test.rs` to verify loading from TOML, YAML, and JSON, and verify CLI argument overrides
    - [ ] Implement `Config` struct and loading logic using the `config` crate (merging default, file, env vars, and CLI arguments via `clap`)
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

## Phase 3: MCP Server Integration (Dual Mode)

- [ ] Task: Set up MCP Server Structure & CLI Args
    - [ ] Implement `clap` arguments to select between `stdio` and `http` modes (and specify config path)
    - [ ] Create basic server traits/structs to abstract the transport layer
- [ ] Task: Implement Stdio Transport
    - [ ] Integrate `mcp_sdk_rs` (or equivalent) for stdio
    - [ ] Write test/verification script
- [ ] Task: Implement HTTP Transport
    - [ ] Set up `axum` router for MCP (SSE endpoint, POST endpoint)
    - [ ] Write test/verification script
- [ ] Task: Register `list_torrents` Tool
    - [ ] Implement the `Tool` trait/interface for `list_torrents`
    - [ ] Connect the tool execution to `QBitClient::get_torrent_list`
    - [ ] Verify tool availability in both transports
- [ ] Task: Update README
    - [ ] Create/Update `README.md` with installation, configuration (file formats), and usage instructions (stdio vs http)
- [ ] Task: Conductor - User Manual Verification 'MCP Server Integration' (Protocol in workflow.md)