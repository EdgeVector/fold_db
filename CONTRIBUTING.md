# Contributing to DataFold

Thank you for your interest in contributing to DataFold! This document provides guidelines and information for contributors.

## Getting Started

### Prerequisites

- Rust 1.70+ with Cargo
- Node.js 18+ (for frontend development)
- AWS credentials (optional, for cloud features)

### Development Setup

```bash
# Clone the repository
git clone https://github.com/shiba4life/fold_db.git
cd fold_db

# Build the project
cargo build

# Run tests
cargo test --lib

# Start the development server
./run.sh
```

### Project Structure

- `src/` - Rust source code
- `src/server/static-react/` - React frontend
- `tests/` - Integration tests
- `docs/` - Documentation
- `examples/` - Example code

## How to Contribute

### Reporting Bugs

1. Check existing [issues](https://github.com/shiba4life/fold_db/issues) to avoid duplicates
2. Use the bug report template when creating a new issue
3. Include steps to reproduce, expected behavior, and actual behavior
4. Include relevant logs and system information

### Suggesting Features

1. Check existing issues and discussions for similar suggestions
2. Use the feature request template
3. Explain the use case and why the feature would be valuable

### Submitting Code

1. Fork the repository
2. Create a feature branch from `mainline`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. Make your changes following the coding standards below
4. Write or update tests as needed
5. Run the test suite:
   ```bash
   cargo test --lib
   cargo clippy
   cd src/server/static-react && npm test
   ```
6. Commit with clear, descriptive messages
7. Push and open a pull request

## Coding Standards

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- Pass all clippy lints (`cargo clippy`)
- No silent failures - always return or propagate errors
- No inline crate imports - use header imports
- Avoid unnecessary branching logic
- Don't create fallbacks that hide broken code
- Write tests for new functionality

### TypeScript/React

- Follow ESLint rules (`npm run lint`)
- Use TypeScript strict mode
- Use the existing API client patterns in `src/api/`
- Write tests using Vitest

### Commits

- Use clear, descriptive commit messages
- Reference issue numbers when applicable (e.g., "Fix #123: description")
- Keep commits focused on a single change

## Testing

### Running Tests

```bash
# Rust tests
cargo test --lib                         # All library tests
cargo test test_name -- --nocapture      # Single test with output

# Frontend tests
cd src/server/static-react
npm test                                 # Run all tests
npm run test:watch                       # Watch mode
```

### Writing Tests

- Place Rust tests in `tests/` or as `#[cfg(test)]` modules
- Place frontend tests alongside components with `.test.tsx` extension
- Test both success and error paths

## Pull Request Process

1. Ensure all tests pass
2. Update documentation if needed
3. Fill out the PR template completely
4. Request review from maintainers
5. Address review feedback
6. Squash commits if requested

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you agree to uphold this code.

## Questions?

- Open a [discussion](https://github.com/shiba4life/fold_db/discussions) for general questions
- Check existing documentation in `docs/`

## License

By contributing, you agree that your contributions will be licensed under the same MIT OR Apache-2.0 license as the project.
