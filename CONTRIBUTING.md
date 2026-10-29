# Contributing to Xylum

First off, thank you for considering contributing to Xylum! It's people like you that make open source such a fantastic community.

As an early-stage project, there are many ways to help, from writing code to finding bugs and improving documentation.

## Code of Conduct

By participating in this project, you are expected to uphold our Code of Conduct (be excellent to each other, be respectful, and collaborate constructively).

## How Can I Contribute?

### Reporting Bugs
This section guides you through submitting a bug report for Xylum. Following these guidelines helps maintainers and the community understand your report, reproduce the behavior, and find related reports.

- Use the **Issues** tab to search for existing bugs. If you find one that matches yours, add a comment instead of opening a new one.
- If you can't find an open issue addressing the problem, open a new one. Be sure to include a **title and clear description**, as much as relevant information, and a **code sample or executable test case** demonstrating the expected behavior that is not occurring.
- Include your OS, Rust version (`rustc --version`), and GPU details if the bug is rendering-related.

### Suggesting Enhancements
- Open an issue with the tag `enhancement`.
- Explain why this enhancement would be useful to most Xylum users and the X11 ecosystem as a whole.

### Pull Requests
- Fill in the required template.
- Do not include issue numbers in the PR title.
- Include screenshots and animated GIFs in your pull request whenever possible.
- Follow the **Rust coding style** (`cargo fmt`).
- End all files with a newline.
- Avoid platform-specific code unless absolutely necessary.

## Development Setup

To get started with Xylum development, ensure you have the following installed:

1.  **Rust**: Make sure you have the latest stable Rust toolchain (Edition 2024). We recommend using `rustup`.
2.  **X11 Utilities**: You'll need tools like `xauth`, `xeyes`, `xterm`, etc., to test the server.
3.  **Linux Environment**: Xylum is currently developed and tested primarily on Linux.

### Building and Testing

```bash
# Build the workspace
cargo build

# Run the test suite (when available)
cargo test

# Run the server with debug logs
cargo run -p xylum-server
```

## Style Guide

### Rust Code
- Use standard Rust formatting: `cargo fmt`.
- Run `cargo clippy` to catch common mistakes and improve your code.
- Prefer explicit error handling over `unwrap()` or `expect()` in library code (it's fine in tests or `main.rs` for early development).
- Write doc comments (`///`) for public APIs.

### Git Commit Messages
We follow **Conventional Commits** to keep the history clean and automated. Please format your commit messages like this:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: A new feature
- `fix`: A bug fix
- `docs`: Documentation only changes
- `style`: Changes that do not affect the meaning of the code (white-space, formatting, etc.)
- `refactor`: A code change that neither fixes a bug nor adds a feature
- `perf`: A code change that improves performance (e.g., MIT-SHM!)
- `test`: Adding missing tests or correcting existing tests
- `chore`: Changes to the build process or auxiliary tools

**Examples:**
- `feat(protocol): add ShmPutImage request`
- `fix(dispatch): correctly handle negative window coordinates`
- `docs(readme): update build instructions`

## License

By contributing to Xylum, you agree that your contributions will be licensed under its **GPL-3.0-or-later** License.