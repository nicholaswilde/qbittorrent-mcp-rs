# Specification: MCP Best Practices & Rules of Engagement

## Overview
This track aims to implement and enforce behavioral "Rules of Engagement" for AI agents using the qBittorrent MCP server. By codifying these best practices, we ensure that agents interact with the torrent client safely, semantically, and securely.

## Functional Requirements
- **MCP Rules Documentation:** Create `conductor/mcp-rules.md` and update `conductor/product-guidelines.md` with adapted rules for qBittorrent (State verification, Destructive confirmation, Search etiquette).
- **Tool Description Updates:** Review and update the descriptions of all tools in `src/server/mcp.rs` to include behavioral hints (e.g., "Verify torrent exists via list_torrents before calling this").
- **MCP Prompt Templates:** Implement an MCP `prompts` resource that provides a "System Directive" for agents, outlining the rules of engagement for managing torrents.
- **Destructive Action Safeguards:** Ensure tools like `delete_torrent` and `shutdown_app` explicitly mention the need for user confirmation in their descriptions.

## Non-Functional Requirements
- **Consistency:** Ensure the rules documented in Conductor files match the instructions provided via MCP prompts.
- **Token Efficiency:** Keep prompt templates concise to minimize overhead for the LLM.

## Acceptance Criteria
- [ ] `conductor/mcp-rules.md` is created with adapted qBittorrent rules.
- [ ] `conductor/product-guidelines.md` is updated.
- [ ] `src/server/mcp.rs` tool metadata contains behavioral safety instructions.
- [ ] The MCP server exposes a `rules-of-engagement` prompt that agents can retrieve.
- [ ] All destructive tools (`delete`, `shutdown`, etc.) have explicit confirmation warnings in their metadata.

## Out of Scope
- Hard-coded logic within the Rust server to block actions without "proof" of state check (the enforcement is via prompt/metadata, not strict state machines).
