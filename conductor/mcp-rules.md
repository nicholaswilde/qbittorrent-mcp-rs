# MCP Rules of Engagement for qBittorrent

This document defines the behavioral rules and best practices for AI agents interacting with the qBittorrent MCP server.

## 1. State Verification
Before performing any action on a specific torrent or resource, the agent **MUST** verify its current state.
- **Rule:** Call `get_torrents` or `get_torrent_info` before calling `pause_torrent`, `resume_torrent`, `delete_torrent`, etc.
- **Goal:** Prevent "blind" commands and handle cases where the torrent might have been deleted or moved.

## 2. Destructive Actions & Confirmation
Actions that result in data loss or service interruption require explicit user awareness.
- **Rule:** The tools `delete_torrent` (especially with `delete_files: true`) and `shutdown_app` **MUST** be preceded by an explanation to the user of what is about to happen.
- **Rule:** For `delete_torrent`, clearly state if files will be deleted from disk.

## 3. Search Etiquette
Search operations in qBittorrent are asynchronous and can be resource-intensive.
- **Rule:** When using `start_search`, do not immediately assume results are available.
- **Rule:** Use `get_search_results` with a reasonable delay (e.g., 2-5 seconds) and polling interval.
- **Rule:** Always call `stop_search` when the desired result is found to conserve server resources.

## 4. Idempotency
Avoid sending redundant commands that don't change the system state.
- **Rule:** If a torrent is already paused, do not call `pause_torrent`.
- **Rule:** If a category already exists, do not attempt to create it again without checking.

## 5. Semantic Feedback
When reporting the result of an MCP tool call to the user, translate technical success/failure into meaningful context.
- **Example:** Instead of "Tool returned success," say "Successfully paused the Debian ISO torrent."

## 6. Security & Privacy
- **Rule:** Never expose or log the WebUI password or sensitive session cookies.
- **Rule:** Be cautious when adding torrents from untrusted URLs.
