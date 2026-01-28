# Specification: Comprehensive Docker Integration Testing

## Overview
Create a new, comprehensive integration test suite that utilizes `testcontainers` to spin up a live `linuxserver/qbittorrent` instance. This suite will verify 100% of the MCP tools' functionality in a realistic environment, ensuring that the server interacts correctly with the actual qBittorrent WebAPI.

## Functional Requirements
- **Container Setup:** Utilize the `testcontainers` Rust crate to manage the `linuxserver/qbittorrent` image.
- **Suite Lifecycle:** Spin up a single container instance for the entire test suite to optimize performance.
- **Dynamic Networking:** Dynamically resolve the WebUI port to avoid port conflicts.
- **Full Coverage Testing:** Exercise every tool exposed by the MCP server, including:
    - **Torrent Management:** Add, pause, resume, stop, start, delete, recheck, reannounce.
    - **Information Retrieval:** List torrents, properties, files, trackers.
    - **Category & Tag Management:** Create, list, and assign categories/tags.
    - **Search Functionality:** Search plugins, start/stop/delete search jobs.
    - **RSS Management:** Add feeds, list items, set/get rules.
    - **Global Controls:** Speed limits, transfer info, alternative speed toggle.
    - **System Inspection:** Logs (main/peer), build info, app version, preferences.
- **Validation:** Assert that each tool returns valid data and that state-changing operations (like adding a torrent) are reflected in subsequent queries.

## Non-Functional Requirements
- **Isolation:** Tests should not depend on any pre-existing local qBittorrent installation.
- **Reliability:** The test suite must be robust enough to run in CI environments.
- **Performance:** Reusing the container across the suite minimizes overhead.

## Acceptance Criteria
- A new integration test file (e.g., `tests/docker_integration_test.rs`) exists.
- Running `cargo test --test docker_integration_test` (or equivalent) passes all tests.
- 100% of the tools defined in `src/server/mcp.rs` are covered by at least one test case.
- All containers are automatically destroyed upon test completion.

## Out of Scope
- Performance benchmarking of the qBittorrent API.
- Testing older qBittorrent API versions (focus is on the latest compatible version).
