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

## Architecture Overview

```
src/
├── fold_node/           # FoldNode — top-level orchestrator (DB + security + config)
│   └── node.rs          # FoldNode struct, clone-friendly Arc wrapper
├── fold_db_core/        # Core database logic (FoldDb, schema manager)
│   └── fold_db.rs
├── schema/              # Schema system
│   ├── core.rs          # SchemaManager — load, approve, query schemas
│   └── types/           # Schema, Field, Query, Mutation, Transform, KeyConfig
├── storage/             # Storage backends
│   ├── traits.rs        # KvStore trait — unified async interface
│   ├── sled_store.rs    # Local embedded storage
│   └── dynamo_store.rs  # AWS DynamoDB backend (behind `aws-backend` feature)
├── handlers/            # Framework-agnostic request handlers
│   ├── mod.rs           # Shared handler layer (used by HTTP server AND Lambda)
│   └── ingestion.rs     # Ingestion-specific handlers
├── ingestion/           # AI-powered data ingestion pipeline
│   ├── ingestion_service/  # Main service (schema recommendation, mutation generation)
│   ├── anthropic_service.rs # Cloud LLM client (https://api.anthropic.com)
│   ├── ollama_service.rs    # Local LLM client (http://127.0.0.1:11434)
│   ├── error.rs         # IngestionError with LLM error classifiers
│   └── routes.rs        # HTTP routes (Actix-web)
├── server/
│   ├── http_server.rs   # Actix-web server setup and route registration
│   └── static-react/    # React frontend (Vite + TypeScript + Redux)
└── error.rs             # Top-level FoldDbError
```

### Key Patterns

- **Handler pattern**: Handlers in `src/handlers/` take typed requests and return typed responses. They're shared between the HTTP server (`http_server.rs`) and AWS Lambda (`exemem-infra/lambdas/`). Keep handlers framework-agnostic — no `actix_web` or `lambda_http` types.

- **KvStore trait**: All storage operations go through the `KvStore` trait (`src/storage/traits.rs`). Local mode uses `SledStore`; cloud mode uses `DynamoStore`. Never call storage backends directly.

- **Schema lifecycle**: Schemas flow through `Pending` -> `Approved` states. New schemas are created via the schema service, loaded into the local `SchemaManager`, then approved. Mutations can only write to approved schemas.

## Writing Tests

### Rust unit tests

Place tests in the same file using `#[cfg(test)] mod tests`. Run with:

```bash
cargo test --lib                          # All unit tests
cargo test --lib ingestion::error::tests  # Specific module
cargo test test_name -- --nocapture       # Single test with output
```

### Integration tests

Integration tests live in `tests/`. Use `test://mock` as the `schema_service_url` to avoid hitting the real schema service:

```rust
let config = FoldNodeConfig {
    schema_service_url: "test://mock".to_string(),
    // ...
};
```

### Git LFS fixtures

Test fixtures like `tests/fixtures/tweets.js` are stored in Git LFS. Run `git lfs pull` after cloning. Without this, fixture files will be pointer stubs and related tests will fail.

### Frontend tests

```bash
cd src/server/static-react
npm test            # Run vitest once
npm run test:watch  # Watch mode
npm run lint        # ESLint
```

Frontend tests use Vitest. Follow existing patterns in `src/__tests__/` for component and Redux slice tests.

## Debugging Tips

### Log levels

FoldDB uses feature-scoped logging. Set the log level with:

```bash
FOLD_LOG_LEVEL=debug ./run.sh --local
```

Available levels: `trace`, `debug`, `info` (default), `warn`, `error`.

Logs are tagged by feature (e.g., `[Ingestion]`, `[HttpServer]`, `[Schema]`). To filter:

```bash
FOLD_LOG_LEVEL=debug ./run.sh --local 2>&1 | grep '\[Ingestion\]'
```

### Common issues

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| "Schema not found" on mutation | Schema not approved | Call `schema_manager.approve("name")` |
| "Anthropic service not initialized" | Missing API key | Configure your Anthropic API key in **Settings → AI Provider** (or switch the provider to Ollama) |
| Frontend embed error at build | Missing `dist/` | `cd src/server/static-react && npm ci && npm run build` |
| Test fixture is 130 bytes | Git LFS pointer | `git lfs pull` |

### Useful commands

```bash
# Check all schemas in the database
curl http://localhost:9001/api/schema/list | jq

# Inspect a specific schema
curl http://localhost:9001/api/schema/get/SCHEMA_NAME | jq

# Check ingestion service status
curl http://localhost:9001/api/ingestion/status | jq

# Watch ingestion progress
curl http://localhost:9001/api/ingestion/progress | jq
```

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
