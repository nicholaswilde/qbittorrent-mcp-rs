# Implementation Plan: Feature Expansion & Polish

## Phase 1: Proactive Notifications (Real-time Event Loop)
- [ ] Task: Update `McpState` or `McpServer` to support a background polling task.
- [ ] Task: Implement `start_event_loop` to poll `api/v2/sync/maindata` at a configurable interval.
- [ ] Task: Implement change detection logic to identify when a torrent finishes downloading.
- [ ] Task: Push JSON-RPC notifications (`notifications/message` or `notifications/resources/updated`) when events occur.
- [ ] Task: Add `polling_interval_ms` to configuration (config file, env var, CLI).
- [ ] Task: Conductor - User Manual Verification 'Phase 1' (Protocol in workflow.md)

## Phase 2: Expanded Prompts Library
- [ ] Task: Add `analyze_disk_space` prompt to help users check storage vs downloads.
- [ ] Task: Add `optimize_speed` prompt to check global limits and connection status.
- [ ] Task: Add `troubleshoot_connection` prompt to diagnose firewalled status or low DHT nodes.
- [ ] Task: Conductor - User Manual Verification 'Phase 2' (Protocol in workflow.md)

## Phase 3: Documentation & Polish
- [ ] Task: Add "Troubleshooting" section to `README.md` covering Docker connectivity and Authentication.
- [ ] Task: Verify and update the "Features" list in `README.md` to match reality.
- [ ] Task: Conductor - User Manual Verification 'Phase 3' (Protocol in workflow.md)
