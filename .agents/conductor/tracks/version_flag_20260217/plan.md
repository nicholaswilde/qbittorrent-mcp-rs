# Implementation Plan: Version Flag Support

This plan outlines the steps to add standard `--version` and `-V` command-line arguments to the `qbittorrent-mcp-rs` binary.

## Phase 1: CLI Configuration and Versioning
Configure `clap` to handle the version flag and provide the necessary metadata.

- [ ] Task: TDD - Create version verification tests.
    - [ ] Create `tests/version_flag_test.rs`.
    - [ ] Implement tests that execute the binary with `--version` and `-V` flags.
    - [ ] Assert that the output matches `qbittorrent-mcp-rs 0.3.5` (or current version from Cargo.toml).
    - [ ] Assert that the process exits with status `0`.
- [ ] Task: Implement versioning in `src/config.rs`.
    - [ ] Update `parse_args` function in `src/config.rs` to include `.version(env!("CARGO_PKG_VERSION"))`.
    - [ ] Ensure the `Command` is configured to automatically handle versioning.
- [ ] Task: Verify tests pass.
    - [ ] Run `cargo test --test version_flag_test`.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: CLI Configuration and Versioning' (Protocol in workflow.md)

## Phase 2: Final Integration and Quality Gate
Ensure the change is robust and adheres to project standards.

- [ ] Task: Verify execution flow.
    - [ ] Manually verify that no configuration files are required or loaded when checking the version.
    - [ ] Manually verify that no log files are created.
- [ ] Task: Quality Gate Check.
    - [ ] Run `task test:ci` to ensure no regressions in configuration parsing or other CLI flags.
    - [ ] Verify that code coverage remains above the 90% project threshold.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Final Integration and Quality Gate' (Protocol in workflow.md)
