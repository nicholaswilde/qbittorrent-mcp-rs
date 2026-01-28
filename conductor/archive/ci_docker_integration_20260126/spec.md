# Specification: CI Docker Integration

## Goal
Integrate the existing `docker_integration_test.rs` suite into the project's GitHub Actions CI workflow to ensure continued stability of the MCP server against a real qBittorrent instance.

## Context
A comprehensive integration test suite using `testcontainers` has been created (`tests/docker_integration_test.rs`). However, it is not currently executed by the CI pipeline.

## Requirements
1.  **Workflow Update**: The `.github/workflows/ci.yml` file must be updated.
2.  **Job Configuration**: A new job or step must be added to run the integration tests.
3.  **Environment**: The tests must run on `ubuntu-latest` to leverage native Docker support.
4.  **Command**: The specific command to run is `cargo test --test docker_integration_test -- --nocapture`.
5.  **Output**: The CI logs should clearly show the test execution and results.

## Acceptance Criteria
-   The CI workflow passes on a fresh push.
-   The logs confirm that `tests/docker_integration_test.rs` was compiled and executed.
-   The tests interact with a `testcontainers` instance of qBittorrent.
