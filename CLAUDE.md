# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Datafold** is a distributed, schema-based database platform with AI-powered data ingestion, designed for personal data sovereignty. It runs both locally (using Sled) and as a serverless AWS application (using DynamoDB + S3).

## Build & Test Commands

```bash
# Build
cargo build                              # Default features
cargo build --features aws-backend       # With AWS backend support

# Lint
cargo clippy                             # Default
cargo clippy --features aws-backend      # With AWS features

# Run Rust tests
cargo test --lib                         # All library tests
cargo test test_name                     # Specific test
cargo test test_name -- --nocapture      # With output

# Run the HTTP server + frontend (port 9001 backend, port 5173 frontend)
# IMPORTANT: Always use ./run.sh to start the UI - never start services manually
./run.sh --local                         # Local mode (Sled storage + global schema service)
./run.sh                                 # Cloud mode (DynamoDB + global schema service)
./run.sh --local --local-schema          # Fully offline (local storage + local schema service)
./run.sh --local --empty-db              # Local with fresh database

# Frontend (in src/server/static-react/)
npm install
npm test                                 # Run vitest tests
npm run test:watch                       # Watch mode
npm run dev                              # Dev server on :5173
npm run lint                             # ESLint
npm run generate:api                     # Generate TypeScript types from OpenAPI spec
```

## Environment Variables

Required for AI-powered ingestion:
```bash
export FOLD_OPENROUTER_API_KEY=your_key  # Or OPENROUTER_API_KEY
```

For cloud mode, ensure AWS credentials are configured (via environment or IAM role).

## Key Architecture Concepts

### Dual Storage Backends
- **Local mode**: Sled embedded key-value store
- **Cloud mode** (`aws-backend` feature): DynamoDB (11 tables) + S3, multi-tenant with user isolation via partition keys

The `KvStore` trait (`src/storage/traits.rs`) provides a unified async interface for both backends:
```rust
#[async_trait]
pub trait KvStore: Send + Sync {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>>;
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()>;
    // ...
}
```

### Schema System
- Schemas define data structure with fields, permissions, transforms
- States flow: `Pending` → `Approved` (via backfill process)
- Key types in `src/schema/types/`: `Schema`, `Field`, `Query`, `Mutation`, `Transform`

### Handler Pattern
Handlers in `src/handlers/` are framework-agnostic, shared between HTTP server and Lambda:
```rust
pub async fn handle_query(
    query: Query,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<QueryResponse>
```

### Feature Flags
- `aws-backend`: Enables DynamoDB, S3, SNS, SQS support
- `test-utils`: Test utilities for integration tests
- `ts-bindings`: Generates TypeScript type definitions via ts-rs

## Error Types

- `FoldDbError` (`src/error.rs`): Top-level application error with variants for Schema, Database, Permission, Network, etc.
- `SchemaError` (`src/schema/types/errors.rs`): Domain errors for schema operations
- `StorageError` (`src/storage/error.rs`): Storage backend errors
- `HandlerError` (`src/handlers/response.rs`): API response errors

## Coding Standards

- No silent failures - throw errors if anything goes wrong
- No branching logic where avoidable - think harder
- Don't use JSON return types in Rust
- No inline crate imports - import in headers only
- Don't create fallbacks
- Use `TODO` format for incomplete implementations
- Assume all tests were passing before changes

## Key Patterns

### Error Handling
```rust
// Use SchemaError for domain errors
SchemaError::InvalidData(format!("Schema '{}' not found", name))

// Handle lock poisoning
self.data.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
```

### Logging
```rust
use crate::log_feature;
log_feature!(LogFeature::HttpServer, info, "Server started on {}", addr);
```

## Important Files

| File | Purpose |
|------|---------|
| `src/datafold_node/node.rs` | Main DataFoldNode - combines DB, security, config |
| `src/fold_db_core/fold_db.rs` | Core database logic |
| `src/storage/traits.rs` | `KvStore` trait abstraction for storage backends |
| `src/handlers/mod.rs` | Shared handler layer for HTTP/Lambda |
| `src/server/http_server.rs` | Actix-web HTTP routes |
| `src/schema/core.rs` | Schema management |

## Manual Testing Workflow

1. Run `./run.sh --local --local-schema` (or `./run.sh` for cloud mode)
2. Navigate to http://localhost:5173 (Vite dev server proxies to backend on 9001)
3. Login with `test_user` if needed
4. Press "Reset Database" button, confirm, wait for completion
5. Go to Ingestion tab → click Twitter → click "Process Data"
6. Wait for ingestion and background indexing to complete
7. Go to Native Index Query tab → search for a term
