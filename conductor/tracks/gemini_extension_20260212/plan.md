# Implementation Plan: Gemini CLI Extension for qBittorrent MCP

This plan outlines the steps required to package and expose the `qbittorrent-mcp-rs` server as a Gemini CLI extension, enabling seamless installation and command-line interaction.

## Phase 1: Extension Scaffolding and Manifest
Establish the foundational extension structure required by the Gemini CLI.

- [x] Task: Research Gemini CLI extension manifest requirements and directory structure.
- [x] Task: Create the extension manifest file (e.g., `gemini-extension.json` or as specified by docs).
- [~] Task: Define the `qbittorrent` command namespace and root entry point.
- [ ] Task: Conductor - User Manual Verification 'Phase 1: Extension Scaffolding and Manifest' (Protocol in workflow.md)

## Phase 2: Binary Management and Installation Logic
Implement the "Self-contained" binary delivery mechanism.

- [ ] Task: Create a script/utility to detect host OS and architecture.
- [ ] Task: Implement a "post-install" or "first-run" hook to download the appropriate `qbittorrent-mcp-rs` binary from GitHub Releases.
- [ ] Task: Ensure the extension can locate and execute the downloaded binary.
- [ ] Task: Conductor - User Manual Verification 'Phase 2: Binary Management and Installation Logic' (Protocol in workflow.md)

## Phase 3: Configuration and Setup Workflow
Enable users to configure their qBittorrent connection details via the CLI.

- [ ] Task: Implement the `gemini qbittorrent setup` command to interactively collect host, port, username, and password.
    - [ ] Task: Provide clear descriptions for each prompt (e.g., "The URL of your qBittorrent Web UI").
    - [ ] Task: Use concise, user-friendly names for settings.
- [ ] Task: Store configuration securely using the Gemini CLI's preferred storage mechanism.
    - [ ] Task: Ensure passwords and sensitive data use `"sensitive": true`.
    - [ ] Task: Support workspace-scoped settings for project-specific qBittorrent instances.
- [ ] Task: Verify that the stored configuration is correctly passed to the MCP server binary during execution.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: Configuration and Setup Workflow' (Protocol in workflow.md)

## Phase 4: Command Mapping and Tool Exposure
Expose all MCP tools as direct subcommands.

- [ ] Task: Map "Torrent Management" tools to `gemini qbittorrent ...` (list, add, pause, etc.).
- [ ] Task: Map "Search" tools to `gemini qbittorrent search ...`.
- [ ] Task: Map "RSS & Categories" tools to their respective subcommands.
- [ ] Task: Implement argument forwarding from the Gemini CLI to the MCP server's tool calls.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Command Mapping and Tool Exposure' (Protocol in workflow.md)

## Phase 5: Final Verification and Documentation
Ensure the extension is robust and well-documented.

- [ ] Task: Test the full installation flow on at least one clean environment.
- [ ] Task: Update the project `README.md` with Gemini CLI installation instructions.
- [ ] Task: Verify all acceptance criteria from `spec.md` are met.
- [ ] Task: Conductor - User Manual Verification 'Phase 5: Final Verification and Documentation' (Protocol in workflow.md)
