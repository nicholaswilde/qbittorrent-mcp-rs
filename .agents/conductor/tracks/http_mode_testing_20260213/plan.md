# Implementation Plan: HTTP Mode Integration Testing

This plan outlines the steps to verify the HTTP transport layer of the qBittorrent MCP server.

## Phase 1: Test Scaffolding and Basic Session
Establish the foundation for HTTP integration testing.

- [ ] Task: Create `tests/http_integration_test.rs` with basic `axum` and `reqwest` setup.
- [ ] Task: Implement a helper to spin up a mock `McpServer` on an ephemeral HTTP port.
- [ ] Task: Verify successful connection to `/sse` and reception of the `endpoint` event.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Test Scaffolding' (Protocol in workflow.md)

## Phase 2: Request/Response Integration
Verify the full JSON-RPC cycle over HTTP/SSE.

- [ ] Task: Implement a test for the `initialize` method cycle.
- [ ] Task: Implement a test for `tools/list` ensuring the response is received via SSE.
- [ ] Task: Implement a test for `tools/call` (e.g., `list_torrents`) and verify the response structure.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Request/Response Integration' (Protocol in workflow.md)

## Phase 3: Authentication and Security
Verify the security middleware.

- [ ] Task: Implement tests for rejected requests (401) when a token is required but missing or invalid.
- [ ] Task: Implement tests for successful access using the `Authorization: Bearer <token>` header.
- [ ] Task: Implement tests for successful access using the `?token=<token>` query parameter.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Authentication and Security' (Protocol in workflow.md)

## Phase 4: Finalization and CI
Ensure robustness and integration.

- [ ] Task: Verify that sessions are correctly cleaned up when connections are dropped (if possible to test).
- [ ] Task: Run the full CI suite `task test:ci` to ensure no regressions.
- [ ] Task: Verify that all acceptance criteria from `spec.md` are fulfilled.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Finalization and CI' (Protocol in workflow.md)
