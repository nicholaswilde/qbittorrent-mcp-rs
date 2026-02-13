# Implementation Plan: Create code coverage using lcov and llvm-cov

This plan outlines the steps to integrate local code coverage reporting using `cargo-llvm-cov`.

## Phase 1: Setup and Documentation
Prepare the project environment and provide developer instructions.

- [ ] Task: Update `.gitignore` to exclude coverage-related artifacts.
    - [ ] Add `lcov.info`.
    - [ ] Add `target/llvm-cov/` or other coverage directories.
- [ ] Task: Update `README.md` with coverage instructions.
    - [ ] Add installation command for `cargo-llvm-cov`.
    - [ ] Document how to run the `task coverage` command.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Setup and Documentation' (Protocol in workflow.md)

## Phase 2: Taskfile Integration
Add the automated coverage generation task to the project's Taskfile.

- [ ] Task: Implement the `coverage` task in `Taskfile.yml`.
    - [ ] Add a command to run `cargo llvm-cov` for a console summary.
    - [ ] Add a command to generate the `lcov.info` file.
    - [ ] Ensure the task excludes build dependencies and focuses on `src/`.
- [ ] Task: Verify the coverage task manually.
    - [ ] Run `task coverage` and confirm the terminal output.
    - [ ] Confirm the creation and content of `lcov.info`.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Taskfile Integration' (Protocol in workflow.md)
