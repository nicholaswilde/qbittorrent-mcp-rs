# Implementation Plan: GitHub Release Summary Command

This plan outlines the steps to create the `release:summary` command, which generates a categorized summary of changes for a release based on `git log`.

## Phase 1: Command Structure and Range Detection [checkpoint: 4a52f1a]
Set up the command file and implement the logic to identify the commit range to summarize.

- [x] Task: Initialize directory structure and `.gemini/commands/release-summary.toml`.
- [x] Task: Implement range detection logic.
    - [x] Sub-task: Write tests for automatic range detection (latest tag vs. previous tag).
    - [x] Sub-task: Implement logic to find the latest and previous tags using `git tag --sort=-v:refname`.
    - [x] Sub-task: Write tests for manual range argument handling.
    - [x] Sub-task: Implement logic to accept a user-provided range or tag.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Command Structure and Range Detection' (Protocol in workflow.md)

## Phase 2: Log Parsing and Categorization [checkpoint: 4fbcc8d]
Implement the parsing of `git log` and the categorization of commits into features, fixes, etc.

- [x] Task: Implement commit parsing logic.
    - [x] Sub-task: Write tests for parsing various commit message formats (Conventional Commits).
    - [x] Sub-task: Implement parsing and categorization into `feat`, `fix`, `chore`, `docs`, etc.
- [x] Task: Implement emoji mapping and categorization.
    - [x] Sub-task: Write tests for emoji shortcode mapping based on `conductor/workflow.md`.
    - [x] Sub-task: Implement mapping (e.g., `feat` -> `:sparkles:`, `fix` -> `:bug:`, etc.).
- [x] Task: Conductor - User Manual Verification 'Phase 2: Log Parsing and Categorization' (Protocol in workflow.md)

## Phase 3: Formatting and Integration [checkpoint: 3764993]
Generate the final Markdown output and integrate it into the `gemini-cli` command.

- [x] Task: Implement Markdown generation.
    - [x] Sub-task: Write tests for Markdown formatting (ensuring triple backticks for code, no line numbers).
    - [x] Sub-task: Implement final string assembly.
- [x] Task: Finalize the `release:summary` command TOML.
    - [x] Sub-task: Define the `description`, `parameters`, and `prompt` in the TOML file.
- [x] Task: Documentation update.
    - [x] Sub-task: Update `README.md` to include instructions for the new `release:summary` command.
- [ ] Task: Quality Gate Check.
    - [ ] Sub-task: Verify total project coverage and adherence to all project standards.
- [x] Task: Conductor - User Manual Verification 'Phase 3: Formatting and Integration' (Protocol in workflow.md)
