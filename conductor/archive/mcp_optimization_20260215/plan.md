# Implementation Plan - MCP Best Practices & Token Optimization

This plan outlines the consolidation of qBittorrent MCP tools to reduce token usage and improve AI interaction efficiency.

## Phase 1: Unified Torrent Management
Consolidate individual torrent action tools into a single, robust management tool.

- [x] Task: TDD - Unified Management Logic
    - [x] Create unit tests in `src/server/mcp.rs` for the new `manage_torrents` handler.
    - [x] Test various actions: `pause`, `resume`, `set_category`, `add_tags`, `set_speed_limits`.
- [x] Task: Implement `manage_torrents` Tool
    - [x] Define the `manage_torrents` tool schema with concise parameter descriptions.
    - [x] Implement the tool handler in `McpServer::call_tool`.
    - [x] Map actions to existing `QBitClient` methods.
- [x] Task: Integration Testing - `manage_torrents`
    - [x] Create `tests/integration_manage_torrents.rs` using `testcontainers`.
    - [x] Verify multi-action sequences against a real qBittorrent instance.
- [~] Task: Conductor - User Manual Verification 'Phase 1: Unified Torrent Management' (Protocol in workflow.md)

## Phase 2: Consolidated Information Retrieval
Merge inspection and system tools to provide richer context with fewer tool definitions.

- [x] Task: TDD - Inspection and System Info
    - [x] Write unit tests for `inspect_torrent` (merging properties, files, trackers).
    - [x] Write unit tests for `get_system_info` (merging transfer, prefs, version).
    - [x] Write tests for enhanced `list_torrents` (optional inclusion of properties/files).
- [x] Task: Implement Consolidated Retrieval Tools
    - [x] Implement `inspect_torrent` handler and tool definition.
    - [x] Implement `get_system_info` handler and tool definition.
    - [x] Update `list_torrents` handler to support `include_properties` and `include_files`.
- [x] Task: Integration Testing - Retrieval
    - [x] Update existing retrieval integration tests to use the new unified tools.
    - [x] Verify JSON-RPC response structures for correctness and density.
- [~] Task: Conductor - User Manual Verification 'Phase 2: Consolidated Information Retrieval' (Protocol in workflow.md)

## Phase 3: Prompt Optimization and Cleanup
Refactor prompts and prune redundant tool definitions to minimize token footprint.

- [x] Task: Refactor Troubleshooting Prompts
    - [x] Consolidate `fix_stalled_torrent`, `optimize_speed`, and `troubleshoot_connection` into `troubleshoot_torrent`.
    - [x] Update prompt logic to handle different inputs (hash vs. general instance issues).
- [x] Task: Description Refinement and Pruning
    - [x] Audit all remaining tool and parameter descriptions for brevity.
    - [x] Remove redundant one-to-one tools (e.g., `pause_torrent`, `get_app_version`) from `McpServer`.
    - [x] Update `tools/list` logic to return the optimized set.
- [x] Task: Final Verification
    - [x] Run `task test:ci` to ensure no regressions.
    - [x] verify code coverage for new handlers exceeds 80%.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Prompt Optimization and Cleanup' (Protocol in workflow.md)
