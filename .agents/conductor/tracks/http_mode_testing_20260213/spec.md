# Specification: HTTP Mode Integration Testing

## Overview
The goal of this track is to implement comprehensive integration tests for the qBittorrent MCP server's HTTP transport mode. This ensures that the SSE-based communication and JSON-RPC message handling over HTTP are robust and secure.

## Functional Requirements
- **Session Lifecycle:** Verify that a GET request to `/sse` initiates a valid session and returns an `endpoint` event.
- **Message Handling:** Verify that JSON-RPC requests sent via POST `/message` are correctly processed and their responses are delivered via the corresponding SSE stream.
- **Authentication:** Ensure that the `auth_middleware` correctly validates tokens in both the `Authorization` header and the `token` query parameter.
- **Concurrency:** Ensure the server can handle multiple concurrent SSE sessions without cross-talk.
- **Error States:** Verify that invalid sessions, missing tokens, and malformed JSON-RPC requests return the appropriate HTTP or JSON-RPC errors.

## Acceptance Criteria
- [ ] A new integration test file `tests/http_integration_test.rs` is implemented.
- [ ] Tests cover successful session initialization and endpoint discovery.
- [ ] Tests cover full request/response cycles for at least one MCP tool.
- [ ] Tests cover both Header and Query Parameter authentication modes.
- [ ] Tests cover unauthorized access scenarios (401 status code).
- [ ] All tests pass consistently using `cargo test`.

## Non-Functional Requirements
- **Isolation:** Tests must use ephemeral ports to avoid conflicts.
- **Coverage:** Aim for 100% path coverage within `src/server/http.rs`.
