# Contributing to CodeCortex

Thank you for your interest in contributing to CodeCortex. This document provides guidelines for contributing.

## Code of conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code.

## How to contribute

### Reporting bugs

- Use the [bug report template](.github/ISSUE_TEMPLATE/bug_report.md) when opening an issue
- Include steps to reproduce, expected vs actual behavior, and environment details
- Check existing issues to avoid duplicates

### Suggesting features

- Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.md)
- Describe the use case and proposed solution
- Consider whether it fits the project scope (code intelligence, MCP, graph indexing)

### Pull requests

1. **Fork and clone** the repository
2. **Create a branch** from `main`: `git checkout -b fix/your-change` or `feat/your-feature`
3. **Make changes** following project conventions
4. **Run checks** before submitting:
   ```bash
   nix flake check --print-build-logs
   # or without Nix:
   cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace
   ```
5. **Commit** with clear messages (see [Conventional Commits](https://www.conventionalcommits.org/) if helpful)
6. **Push** and open a pull request using the [PR template](.github/PULL_REQUEST_TEMPLATE.md)

### Development setup

- **Preferred**: Use Nix for a reproducible environment:
  ```bash
  nix develop
  cargo build
  cargo test --workspace
  ```
- **Fallback**: Install Rust, protobuf, pkg-config, cmake, and OpenSSL. See [docs/INSTALL.md](docs/INSTALL.md).

### Code style

- Follow `cargo fmt` and `cargo clippy` output
- Use `Result<T, E>` and explicit error types at fallible boundaries
- Prefer `Arc` and ownership-safe sharing; avoid `unsafe` unless necessary
- Keep async boundaries explicit

### Testing

- Add unit tests for new logic
- Update integration tests if behavior changes
- Run `nix flake check` or `cargo test --workspace` before submitting

## Project structure

- `crates/` — Rust workspace crates
- `docs/` — User and developer documentation
- `.github/workflows/` — CI and release automation

## Questions?

Open a [Discussion](https://github.com/aloshkarev/codecortex/discussions) or an issue if something is unclear.
