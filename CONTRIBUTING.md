# Contributing to FoldDB

Thank you for your interest in contributing to FoldDB! This document provides guidelines and instructions for contributing.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/fold_db.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test --lib`
6. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 16+ (for frontend development)
- AWS credentials (optional, for cloud features)

### Git LFS

Some test fixtures (e.g., `tests/fixtures/tweets.js`) are tracked with Git LFS. To download them:

```bash
git lfs install   # one-time setup
git lfs pull      # download LFS-tracked files
```

Without this, those files will be small pointer files instead of actual data, and related tests will fail.

### Building

```bash
# Build without AWS features
cargo build

# Build with AWS backend support
cargo build --features aws-backend

# Run linter
cargo clippy
cargo clippy --features aws-backend
```

### Running Tests

```bash
# Run all library tests
cargo test --lib

# Run a specific test with output
cargo test test_name -- --nocapture

# Run frontend tests
cd src/server/static-react
npm test
```

### Running Locally

```bash
# Local mode with Sled storage + prod schema service (recommended)
./run.sh --local

# Local mode with dev schema service
./run.sh --local --dev

# Fully offline development (local storage + local schema service)
./run.sh --local --local-schema

# Local mode with fresh empty database
./run.sh --local --empty-db

# Cloud mode (requires AWS credentials)
./run.sh

# Show all options
./run.sh --help
```

The script automatically kills any existing processes before starting.

## Code Style

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- No silent failures - always propagate or handle errors explicitly
- Avoid unnecessary branching logic
- Use `SchemaError` for domain errors
- Import crates in file headers, not inline
- Use `TODO` comments for incomplete implementations

### TypeScript/React

- Run `npm run lint` before committing
- Use TypeScript strict mode
- Follow existing patterns for Redux slices and API clients

## Pull Request Process

1. Ensure all tests pass locally
2. Update documentation if needed
3. Add tests for new functionality
4. Keep PRs focused - one feature or fix per PR
5. Write clear commit messages describing the "why" not just the "what"
6. Reference any related issues in the PR description

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add native index support for range queries

fix: resolve race condition in transform queue

docs: update API documentation for ingestion endpoints

refactor: simplify storage backend trait hierarchy
```

## Reporting Issues

- Use GitHub Issues for bug reports and feature requests
- Search existing issues before creating a new one
- Provide reproduction steps for bugs
- Include relevant logs and error messages

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## Questions?

Open a GitHub Discussion for questions about contributing or the codebase.
