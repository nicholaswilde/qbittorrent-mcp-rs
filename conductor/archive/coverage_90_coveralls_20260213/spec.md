# Specification: Increase code coverage to >90% with Coveralls

## Overview
This track aims to increase the project's code coverage from its current level (~50% total) to above 90% by targeting under-tested areas like the HTTP server and complex tool handlers. It also implements manual reporting to Coveralls.io via LCOV.

## Functional Requirements
- **Comprehensive Coverage Expansion:**
    - Implement integration tests for `server/http.rs` (targeted via "HTTP Mode Integration Testing" track or merged logic).
    - Expand tests for `server/mcp.rs` tool handlers to cover missing paths.
    - Increase edge-case testing in `client.rs`.
- **Coveralls.io Integration:**
    - Add a `task` command to upload the local `lcov.info` file to Coveralls.io using the `coveralls` CLI or a similar script.
    - Require a `COVERALLS_REPO_TOKEN` environment variable for authentication.
- **Reporting:**
    - Maintain the local console summary and `lcov.info` generation implemented in previous tracks.

## Non-Functional Requirements
- **Target Target:** >90% total line coverage.
- **Soft Enforcement:** Coverage metrics should be visible but not block local development or CI until explicitly configured to do so.

## Acceptance Criteria
- [ ] Total project line coverage is at or above 90% as reported by `cargo llvm-cov`.
- [ ] `server/http.rs` has significant coverage (>80%).
- [ ] `Taskfile.yml` is updated with a `coverage:upload` task.
- [ ] Instructions for uploading to Coveralls are added to the `README.md`.
- [ ] A successful manual upload to Coveralls.io is verified.

## Out of Scope
- Automated CI upload (GitHub Actions).
- Hard failure of CI on coverage drops.
- Testing of external crates or build dependencies.
