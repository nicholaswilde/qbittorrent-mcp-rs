# Implementation Plan: Increase code coverage to >90% with Coveralls

This plan outlines the steps to reach >90% code coverage and integrate manual reporting to Coveralls.io.

## Phase 1: HTTP Server Integration Testing [checkpoint: 6213d8a]
Target the 0% coverage area in `server/http.rs`.

- [x] Task: Implement integration tests for the HTTP server transport.
    - [x] Create `tests/http_integration_test.rs`.
    - [x] Write tests for SSE session initialization and endpoint discovery.
    - [x] Write tests for JSON-RPC request/response cycle over HTTP.
    - [x] Write tests for authentication middleware (Header and Query Parameter).
- [x] Task: Verify coverage for `server/http.rs` reaches >80%.
- [x] Task: Conductor - User Manual Verification 'Phase 1: HTTP Server Integration Testing' (Protocol in workflow.md)

## Phase 2: MCP Tool Handler Expansion [checkpoint: de3a1c5]
Target the low coverage (~37%) in `server/mcp.rs`.

- [x] Task: Identify untested tool handlers in `src/server/mcp.rs`.
- [x] Task: Write unit or integration tests for all missing tool paths.
    - [x] Focus on RSS management tools.
    - [x] Focus on Search plugin management tools.
    - [x] Focus on Advanced functionality (mass rename, find duplicates, etc.).
- [x] Task: Verify coverage for `server/mcp.rs` reaches >80%.
- [x] Task: Conductor - User Manual Verification 'Phase 2: MCP Tool Handler Expansion' (Protocol in workflow.md)

## Phase 3: Client Edge Cases and Final Push
Target remaining gaps to reach the 90% total goal.

- [x] Task: Expand `tests/client_test.rs` and related files to cover error paths in `client.rs`.
- [x] Task: Add tests for any remaining uncovered lines in `config.rs` and `main.rs`.
- [x] Task: Run `task coverage` and verify total line coverage is >= 90%.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Client Edge Cases and Final Push' (Protocol in workflow.md)

## Phase 4: Coveralls.io Integration
Implement the manual upload mechanism.

- [x] Task: Add the `coverage:upload` task to `Taskfile.yml`.
    - [x] Use `cargo llvm-cov` to generate `lcov.info`.
    - [x] Implement the upload command using the Coveralls CLI (e.g., `coveralls --lcov lcov.info`).
- [x] Task: Update `README.md` with instructions for Coveralls reporting.
    - [x] Document the need for `COVERALLS_REPO_TOKEN`.
    - [x] Provide the command to run the upload task.
- [x] Task: Perform a successful manual upload and verify on the Coveralls dashboard.
- [x] Task: Conductor - User Manual Verification 'Phase 4: Coveralls.io Integration' (Protocol in workflow.md)
