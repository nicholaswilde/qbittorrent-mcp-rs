# Implementation Plan: Comprehensive Docker Integration Testing

## Phase 1: Infrastructure & Test Harness [checkpoint: f8511a3]
- [x] Task: Add `testcontainers` and `testcontainers-modules` dependencies to `Cargo.toml`.
- [x] Task: Create `tests/docker_integration_test.rs` with the shared container infrastructure (suite-level setup/teardown).
- [x] Task: Verify the harness can successfully pull the image and spin up a container.
- [x] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Core & System Tools Testing
- [x] Task: Write failing tests for System Info tools (`get_app_version`, `get_build_info`, `get_app_preferences`).
- [x] Task: Implement testing logic for system tools and verify they pass (Green).
- [ ] Task: Write failing tests for Global Transfer tools (`get_global_transfer_info`, `set_global_transfer_limits`, `toggle_alternative_speed_limits`).
- [ ] Task: Implement testing logic for global transfer tools and verify they pass (Green).
- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Torrent Lifecycle & Control Testing
- [ ] Task: Write failing tests for Torrent addition (`add_torrent`) and basic listing (`list_torrents`).
- [ ] Task: Implement torrent addition testing logic and verify it passes (Green).
- [ ] Task: Write failing tests for Torrent control (`stop`, `start`, `pause`, `resume`, `recheck`, `reannounce`).
- [ ] Task: Implement torrent control testing logic and verify it passes (Green).
- [ ] Task: Write failing tests for Torrent inspection (`get_torrent_files`, `get_torrent_properties`, `get_torrent_trackers`).
- [ ] Task: Implement torrent inspection testing logic and verify it passes (Green).
- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)

## Phase 4: Metadata, Search & RSS Testing
- [ ] Task: Write failing tests for Category and Tag management.
- [ ] Task: Implement category/tag testing logic and verify it passes (Green).
- [ ] Task: Write failing tests for Search functionality (plugin listing and job lifecycle).
- [ ] Task: Implement search testing logic and verify it passes (Green).
- [ ] Task: Write failing tests for RSS management (feeds and rules).
- [ ] Task: Implement RSS testing logic and verify it passes (Green).
- [ ] Task: Conductor - User Manual Verification 'Phase 4' (Protocol in workflow.md)

## Phase 5: Advanced Controls & Final Polish
- [ ] Task: Write failing tests for per-torrent share and speed limits.
- [ ] Task: Implement limit testing logic and verify it passes (Green).
- [ ] Task: Final pass to ensure every tool defined in `src/server/mcp.rs` is exercised.
- [ ] Task: Conductor - User Manual Verification 'Phase 5' (Protocol in workflow.md)
