# Implementation Plan: MCP Best Practices & Rules of Engagement

This plan details the steps to implement behavioral rules and best practices for AI agents interacting with the qBittorrent MCP server.

## Phase 1: Conductor Documentation Updates [checkpoint: 9f50900]
Codify the rules of engagement within the project's management documents.

- [x] Task: Create `conductor/mcp-rules.md` with adapted rules (Verify state, Semantic feedback, Idempotency, Security).
- [x] Task: Update `conductor/product-guidelines.md` to reference the new rules of engagement.
- [x] Task: Conductor - User Manual Verification 'Phase 1: Conductor Documentation Updates' (Protocol in workflow.md)

## Phase 2: Tool Metadata Enhancement
Update the MCP tool definitions in code to include safety and behavioral instructions.

- [x] Task: Identify all destructive or state-changing tools in `src/server/mcp.rs`.
- [x] Task: Update tool descriptions for `delete_torrent`, `pause_torrent`, `resume_torrent`, and `shutdown_app` with confirmation and state-check requirements.
- [x] Task: Update search-related tool descriptions with etiquette instructions (e.g., polling requirements).
- [~] Task: Conductor - User Manual Verification 'Phase 2: Tool Metadata Enhancement' (Protocol in workflow.md)

## Phase 3: MCP Prompts Implementation
Expose the rules of engagement as a machine-readable prompt resource.

- [ ] Task: Define a new MCP prompt named `rules-of-engagement` in `src/server/mcp.rs`.
- [ ] Task: Implement the logic to return the system directive/best practices text when this prompt is requested.
- [ ] Task: Write unit tests to verify the prompt resource is correctly registered and returns the expected text.
- [ ] Task: Conductor - User Manual Verification 'Phase 3: MCP Prompts Implementation' (Protocol in workflow.md)

## Phase 4: Verification and Final Polish
Ensure all acceptance criteria are met across documentation and code.

- [ ] Task: Run `task test:ci` to ensure no regressions were introduced.
- [ ] Task: Use the MCP Inspector to verify that tool descriptions and the new prompt resource are correctly exposed.
- [ ] Task: Verify that all acceptance criteria from `spec.md` have been fulfilled.
- [ ] Task: Conductor - User Manual Verification 'Phase 4: Verification and Final Polish' (Protocol in workflow.md)
