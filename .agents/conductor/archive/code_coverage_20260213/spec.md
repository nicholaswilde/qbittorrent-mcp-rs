# Specification: Create code coverage using lcov and llvm-cov

## Overview
This track implements local code coverage reporting for developers using `cargo-llvm-cov`. It will provide a quick console summary and an LCOV info file for IDE integration, without impacting the CI pipeline.

## Functional Requirements
- **Local Coverage Generation:** Add a `task` command to run tests with coverage instrumentation.
- **Report Formats:**
    - **Console Summary:** Print a coverage table to the terminal.
    - **LCOV Info:** Generate a `lcov.info` file in the project root or a dedicated directory.
- **Tooling:** Utilize `cargo-llvm-cov` as the primary driver for coverage collection and reporting.

## Non-Functional Requirements
- **Developer Workflow:** The process must be non-intrusive and strictly local.
- **Documentation:** Provide instructions in the `README.md` or a new developer guide on how to install the tool and run the task.

## Acceptance Criteria
- [ ] `Taskfile.yml` is updated with a `coverage` task.
- [ ] Running `task coverage` successfully generates a console summary.
- [ ] Running `task coverage` successfully generates a `lcov.info` file.
- [ ] The generated coverage excludes external dependencies and focus strictly on project source code.
- [ ] `.gitignore` is updated to exclude coverage artifacts (e.g., `coverage/`, `lcov.info`).

## Out of Scope
- Integration with GitHub Actions or other CI providers.
- Generation of HTML coverage reports.
- Automated uploading of coverage data to external services (e.g., Codecov).
