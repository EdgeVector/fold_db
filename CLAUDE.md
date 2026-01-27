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

# Run the HTTP server (port 9001)
./run                                    # Cloud mode (kills existing processes, starts frontend+backend)
cargo run --bin datafold_http_server -- --port 9001

# Frontend (in src/server/static-react/)
npm install
npm test                                 # Run vitest tests
npm run test:watch                       # Watch mode
npm run dev                              # Dev server on :5173
npm run lint                             # ESLint
npm run build                            # Production build
```

## Project Structure

```
src/
├── atom/                 # Atomic data storage units
├── bin/                  # Binary entry points (datafold_http_server)
├── datafold_node/        # Node implementation, config, operation processor
├── db_operations/        # Database operation handlers, native indexing
├── fold_db_core/         # Core database: queries, mutations, orchestration
├── handlers/             # HTTP request handlers (shared between server/lambda)
├── ingestion/            # AI-powered data ingestion pipeline
├── lambda/               # AWS Lambda-specific code
├── logging/              # Multi-output logging system (Web, DynamoDB, Console, File)
├── schema/               # Schema definition, validation, types
├── security/             # Cryptography (Ed25519, AES-GCM, signing)
├── server/               # HTTP server (Actix-web) + React UI
│   └── static-react/     # React frontend (Vite, Redux Toolkit, Tailwind)
├── storage/              # Storage backends (Sled, DynamoDB, S3)
└── transform/            # Data transformation pipeline
```

## Key Architecture Concepts

### Dual Storage Backends
- **Local mode**: Sled embedded key-value store
- **Cloud mode** (`aws-backend` feature): DynamoDB (11 tables) + S3, multi-tenant with user isolation via partition keys

### Schema System
- Schemas define data structure with fields, permissions, transforms
- States flow: `Pending` → `Approved` (via backfill process)
- Supports hash+range key configurations for DynamoDB

### Handler Pattern
Handlers are framework-agnostic, shared between HTTP server and Lambda:
```rust
// In src/handlers/
pub async fn handle_query(
    query: Query,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<QueryResponse>
```

### Feature Flags
- `aws-backend`: Enables DynamoDB, S3, Lambda support
- `lambda`: AWS Lambda runtime support (implies `aws-backend`)
- `ts-bindings`: Generates TypeScript type definitions

## Coding Standards (from .cursorrules)

- No silent failures - throw errors if anything goes wrong
- No branching logic where avoidable - think harder
- Don't use JSON return types in Rust
- No inline crate imports - import in headers only
- Keep imports organized
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
// Simple initialization with fallback
crate::logging::LoggingSystem::init_with_fallback(cloud_config).await;

// Feature-specific logging
use crate::log_feature;
log_feature!(LogFeature::HttpServer, info, "Server started on {}", addr);
```

### Async Pattern
The codebase is fully async using Tokio:
```rust
pub async fn do_something(&self) -> Result<T, SchemaError> {
    // async implementation
}
```

## Important Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library root, exports public API |
| `src/datafold_node/node.rs` | Main DataFoldNode implementation |
| `src/fold_db_core/fold_db.rs` | Core database logic |
| `src/schema/core.rs` | Schema management |
| `src/server/http_server.rs` | HTTP routes configuration |
| `src/logging/mod.rs` | Logging system with `init_with_fallback` |

## Manual Testing Workflow

1. Run `./run`
2. Navigate to http://localhost:9001
3. Login with `test_user` if needed
4. Press "Reset Database" button, confirm, wait for completion
5. Go to Ingestion tab → click Twitter → click "Process Data"
6. Wait for ingestion and background indexing to complete
7. Go to Native Index Query tab → search for a term
