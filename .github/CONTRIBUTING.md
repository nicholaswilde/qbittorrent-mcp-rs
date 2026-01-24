# Contributing to qBittorrent MCP Server (Rust)

First off, thank you for considering contributing to qBittorrent MCP Server (Rust)! It's people like you that make the open-source community such an amazing place.

## How Can I Contribute?

### Reporting Bugs

If you find a bug, please create an issue on GitHub. Include:
- A clear, descriptive title.
- Steps to reproduce the bug.
- The expected behavior and what actually happened.
- Any relevant logs or error messages.
- Your environment details (OS, Rust version, qBittorrent version).

### Suggesting Features

We welcome feature requests! Please open an issue and describe:
- The problem this feature would solve.
- How you imagine the feature working.
- Any alternative solutions you've considered.

### Code Contributions

1. **Fork the repository** and create your branch from `main`.
2. **Install dependencies**: Ensure you have a Rust toolchain and [go-task](https://taskfile.dev/) installed.
3. **Make your changes**: Adhere to the existing code style and conventions.
4. **Test your changes**: Run `task test` to ensure everything is working correctly.
5. **Lint and Format**: Run `task lint` and `task fmt` before submitting.
6. **Commit your changes**: Use clear and descriptive commit messages (Conventional Commits preferred).
7. **Submit a Pull Request**: Provide a clear description of your changes and reference any related issues.

### Working with Conductor

This project uses **Gemini Conductor** to manage development workflow in a structured way. All new features and bug fixes should be implemented as **Tracks**.

1.  **Start a Track**: To start working on a planned track, use the Gemini CLI command:
    ```
    /conductor:implement
    ```
    This will guide you through selecting and implementing tasks from the `plan.md` file.

2.  **Follow the Workflow**: Conductor enforces the project's workflow, including test-driven development, code coverage checks, and phase-based commits. Please follow the prompts provided by the agent.

3.  **Do Not Manually Edit Plans**: Avoid manually editing `conductor/tracks/<track_id>/plan.md` unless instructed. The Conductor agent manages task status and commit linking automatically.

## Development Setup

### Prerequisites

- [Rust](https://rustup.rs/) (Stable)
- [go-task](https://taskfile.dev/) (Optional, but recommended for automation)
- [Docker](https://www.docker.com/) (Optional, for container-related tasks)

### Common Tasks

We use `task` to automate common development operations:

- `task check`: Check for compilation errors.
- `task test`: Run unit tests.
- `task fmt`: Format code using `rustfmt`.
- `task lint`: Run lints using `clippy`.
- `task build:local`: Build the project in release mode for your current architecture.
- `task build:debug`: Build the project in debug mode.

If you don't have `task` installed, you can use the underlying `cargo` commands found in `Taskfile.yml`.

## Pull Request Process

- Ensure the CI suite passes. Our CI runs tests on Linux, macOS, and Windows.
- Keep pull requests focused on a single change or related set of changes.
- Update the `README.md` or other documentation if your change introduces new features or modifies existing ones.
- Adhere to the [Apache License 2.0](LICENSE).

## Style Guidelines

- Follow standard Rust naming conventions (`snake_case` for functions/variables, `PascalCase` for structs/enums).
- Run `cargo fmt` before committing.
- Ensure `cargo clippy` passes without warnings.

Thank you for your contribution!
