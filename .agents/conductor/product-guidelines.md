# Product Guidelines

## Brand & Tone
- **Professional & Robust:** The project values reliability, performance, and technical accuracy.
- **Developer-Centric:** Documentation and interfaces should be clear, precise, and respectful of the user's technical expertise.
- **Safety First:** Emphasize the safety guarantees provided by Rust and the security of the API interactions.

## Rules of Engagement
- **Behavioral Standards:** AI agents interacting with this MCP server are expected to follow the [Rules of Engagement](./mcp-rules.md).
- **State Verification:** Always verify the state of a torrent before attempting modification.
- **Explicit Confirmation:** Destructive actions (deletion, shutdown) require clear user communication.

## Design Principles
- **Efficiency:** The server should be lightweight and performant, minimizing resource overhead.
- **Simplicity:** The MCP interface should be intuitive and easy to integrate with various AI agents.
- **Idiomatic Rust:** Code should follow standard Rust conventions (fmt, clippy) and leverage the type system for correctness.

## Contribution Guidelines
- **Code Quality:** All PRs must pass `cargo test` and `cargo clippy`.
- **Documentation:** New features must include corresponding documentation and examples.
- **Commit Style:** Follow Conventional Commits for clear history.
