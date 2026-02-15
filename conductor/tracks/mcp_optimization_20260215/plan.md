# Implementation Plan - MCP Best Practices & Token Optimization

This plan outlines the consolidation of qBittorrent MCP tools to reduce token usage and improve AI interaction efficiency.

## Phase 1: Unified Torrent Management
Consolidate individual torrent action tools into a single, robust management tool.

- [ ] Task: TDD - Unified Management Logic
    - [ ] Create unit tests in `src/server/mcp.rs` for the new `manage_torrents` handler.
    - [ ] Test various actions: `pause`, `resume`, `set_category`, `add_tags`, `set_speed_limits`.
- [ ] Task: Implement `manage_torrents` Tool
    - [ ] Define the `manage_torrents` tool schema with concise parameter descriptions.
    - [ ] Implement the tool handler in `McpServer::call_tool`.
    - [ ] Map actions to existing `QBitClient` methods.
- [ ] Task: Integration Testing - `manage_torrents`
    - [ ] Create `tests/integration_manage_torrents.rs` using `testcontainers`.
    - [ ] Verify multi-action sequences against a real qBittorrent instance.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Unified Torrent Management' (Protocol in workflow.md)

## Phase 2: Consolidated Information Retrieval
Merge inspection and system tools to provide richer context with fewer tool definitions.

- [ ] Task: TDD - Inspection and System Info
    - [ ] Write unit tests for `inspect_torrent` (merging properties, files, trackers).
    - [ ] Write unit tests for `get_system_info` (merging transfer, prefs, version).
    - [ ] Write tests for enhanced `list_torrents` (optional inclusion of properties/files).
- [ ] Task: Implement Consolidated Retrieval Tools
    - [ ] Implement `inspect_torrent` handler and tool definition.
    - [ ] Implement `get_system_info` handler and tool definition.
    - [ ] Update `list_torrents` handler to support `include_properties` and `include_files`.
- [ ] Task: Integration Testing - Retrieval
    - [ ] Update existing retrieval integration tests to use the new unified tools.
    - [ ] Verify JSON-RPC response structures for correctness and density.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Consolidated Information Retrieval' (Protocol in workflow.md)

## Phase 3: Prompt Optimization and Cleanup
Refactor prompts and prune redundant tool definitions to minimize token footprint.

- [ ] Task: Refactor Troubleshooting Prompts
    - [ ] Consolidate `fix_stalled_torrent`, `optimize_speed`, and `troubleshoot_connection` into `troubleshoot_torrent`.
    - [ ] Update prompt logic to handle different inputs (hash vs. general instance issues).
- [ ] Task: Description Refinement and Pruning
    - [ ] Audit all remaining tool and parameter descriptions for brevity.
    - [ ] Remove redundant one-to-one tools (e.g., `pause_torrent`, `get_app_version`) from `McpServer`.
    - [ ] Update `tools/list` logic to return the optimized set.
- [ ] Task: Final Verification
    - [ ] Run `task test:ci` to ensure no regressions.
    - [ ] verify code coverage for new handlers exceeds 80%.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Prompt Optimization and Cleanup' (Protocol in workflow.md)
