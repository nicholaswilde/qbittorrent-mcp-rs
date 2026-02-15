# Specification - MCP Best Practices & Token Optimization

## Overview
This track aims to optimize the qBittorrent MCP server by adopting best practices for tool design. The primary goal is to reduce token consumption by consolidating multiple one-to-one API mappings into unified, multi-purpose tools and prompts.

## Functional Requirements

### 1. Unified Torrent Management
- Implement a consolidated management tool (e.g., `manage_torrents`) that accepts a `hashes` parameter and an `action` parameter.
- Supported actions must include:
    - **State Changes**: `pause`, `resume`, `reannounce`, `recheck`.
    - **Metadata**: `set_category`, `add_tags`, `remove_tags`.
    - **Limits/Priority**: `set_share_limits`, `set_speed_limits`, `toggle_sequential`, `toggle_first_last_prio`, `set_force_start`, `set_super_seeding`.

### 2. Consolidated Information Retrieval
- Implement `inspect_torrent`: Merges the functionality of `get_torrent_properties`, `get_torrent_files`, and `get_torrent_trackers`.
- Enhance `list_torrents`: Add optional flags to include properties or file lists directly in the listing output to save round-trips.
- Implement `get_system_info`: Combines `get_global_transfer_info`, `get_app_preferences`, `get_app_version`, and `get_build_info`.

### 3. Prompt Optimization
- Consolidate troubleshooting prompts (`fix_stalled_torrent`, `optimize_speed`, `troubleshoot_connection`) into a single, context-aware `troubleshoot_torrent` prompt.
- Refine tool and parameter descriptions to be concise yet descriptive, minimizing unnecessary tokens in the tool definitions.

## Non-Functional Requirements
- **Token Efficiency**: The total size of the tool definition JSON must be significantly reduced.
- **Backward Compatibility**: Ensure that the underlying client methods remain robust and that the new tool logic correctly maps to the existing `QBitClient` implementation.

## Acceptance Criteria
- [ ] `tools/list` returns a smaller, more organized set of tools.
- [ ] All original functionality is preserved and accessible through the consolidated tools.
- [ ] `troubleshoot_torrent` prompt correctly handles different troubleshooting scenarios based on input.
- [ ] All new and modified tools have corresponding unit and integration tests.
- [ ] Code coverage for new logic exceeds 80%.

## Out of Scope
- Adding new qBittorrent features not already present in the client.
- Changing the underlying authentication or connection logic.
