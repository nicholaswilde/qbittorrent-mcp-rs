# Specification: GitHub Release Summary Command

## Overview
Create a `gemini-cli` command named `release:summary` that generates a concise, emoji-categorized summary of changes for a release. This command will primarily use the local `git log` and adhere to project-specific formatting guidelines.

## Functional Requirements
- **Command Name:** `release:summary`
- **Location:** `.gemini/commands/release-summary.toml`
- **Change Detection:**
    - **Automatic Mode:** If no arguments are provided, compare the latest tag with the previous tag.
    - **Manual Mode:** Accept a specific tag or commit range as an argument to summarize changes for that range.
- **Summary Generation:**
    - Parse `git log` output between the detected or provided range.
    - Categorize changes based on commit message prefixes (e.g., `feat`, `fix`, `chore`).
    - Use emoji shortcodes as defined in the project's workflow guidelines (e.g., `:rocket:`, `:bug:`, `:sparkles:`).
    - Ensure output is in standard Markdown with no line numbers and triple backticks for code blocks.
- **Output:**
    - Print the formatted summary directly to the terminal.

## Non-Functional Requirements
- **Efficiency:** Minimize processing time by using local `git` commands.
- **Consistency:** Maintain strict adherence to the "GitHub Release Summary Guidelines" in `conductor/workflow.md`.

## Acceptance Criteria
- [ ] Command file `.gemini/commands/release-summary.toml` is created.
- [ ] `gemini release:summary` correctly identifies and summarizes the diff between the two most recent tags.
- [ ] `gemini release:summary <range>` correctly summarizes changes for the specified range.
- [ ] Output follows all formatting rules (emojis, Markdown, no line numbers).

## Out of Scope
- Authenticating with GitHub or calling the GitHub API.
- Creating the release entry on GitHub.com (this command only generates the text).
