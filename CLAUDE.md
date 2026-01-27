# CLAUDE.md

This file provides context for Claude Code when working on this project.

## Project Overview

**Datafold** is a distributed, schema-based database platform with AI-powered data ingestion, designed for personal data sovereignty. It runs both locally (using Sled) and as a serverless AWS application (using DynamoDB + S3).

## Tech Stack

- **Language:** Rust
- **HTTP Framework:** Actix-web 4.3
- **Async Runtime:** Tokio
- **Local Storage:** Sled (embedded key-value store)
- **Cloud Storage:** AWS DynamoDB + S3
- **AI/LLM:** OpenRouter API (Claude models)
- **Frontend:** React (in `src/server/static-react/`)

## Project Structure

```
src/
├── atom/                 # Atomic data storage units
├── bin/                  # Binary entry points (datafold_http_server, schema_service)
├── datafold_node/        # Node implementation, config, operation processor
├── db_operations/        # Database operation handlers, native indexing
├── fold_db_core/         # Core database: queries, mutations, orchestration
├── handlers/             # HTTP request handlers (shared between server/lambda)
├── ingestion/            # AI-powered data ingestion pipeline
├── lambda/               # AWS Lambda-specific code
├── logging/              # Multi-output logging system
├── schema/               # Schema definition, validation, types
├── schema_service/       # Centralized schema management service
├── security/             # Cryptography (Ed25519, AES-GCM, signing)
├── server/               # HTTP server implementation
├── storage/              # Storage backends (Sled, DynamoDB, S3)
├── transform/            # Data transformation pipeline
└── utils/                # Shared utilities
```

## Build Commands

```bash
# Build (default features)
cargo build

# Build with AWS backend support
cargo build --features aws-backend

# Run tests
cargo test --lib

# Run clippy linter
cargo clippy

# Run clippy with AWS features
cargo clippy --features aws-backend

# Run the HTTP server
cargo run --bin datafold_http_server -- --port 8080
```

## Key Patterns

### Error Handling

Use `SchemaError` for domain errors. The codebase has an `ErrorFactory` in `src/utils/error_factory.rs` for common error patterns.

```rust
// Preferred
SchemaError::InvalidData(format!("Schema '{}' not found", name))

// For lock errors, use unwrap_or_else to handle poisoning
self.data.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
```

### Async/Await

The codebase is fully async. Use `tokio` runtime features.

```rust
pub async fn do_something(&self) -> Result<T, SchemaError> {
    // async implementation
}
```

### Feature Flags

- `aws-backend`: Enables DynamoDB, S3, Lambda support
- `ts-bindings`: Generates TypeScript type definitions

```rust
#[cfg(feature = "aws-backend")]
use aws_sdk_dynamodb::Client;
```

### Logging

Use the `LoggingSystem` for initialization:

```rust
// Simple initialization with fallback
crate::logging::LoggingSystem::init_with_fallback(cloud_config).await;

// Feature-specific logging
use crate::log_feature;
log_feature!(LogFeature::HttpServer, info, "Server started on {}", addr);
```

### Handler Pattern

Handlers are framework-agnostic and shared between HTTP server and Lambda:

```rust
// In src/handlers/
pub async fn handle_query(
    query: Query,
    user_hash: &str,
    node: &DataFoldNode,
) -> HandlerResult<QueryResponse> {
    // Implementation
}
```

## Database Architecture

### Local Mode
- Uses **Sled** embedded database
- Data stored in local directory

### Cloud Mode (aws-backend feature)
- **DynamoDB** for structured data (11 tables)
- **S3** for file storage and large blobs
- Multi-tenant with user isolation via partition keys

### Schema System
- Schemas define data structure with fields, permissions, transforms
- States: `Pending` -> `Approved` (via backfill)
- Supports hash+range key configurations

## Testing

```bash
# Run all library tests
cargo test --lib

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Common Tasks

### Adding a New Handler

1. Create handler function in `src/handlers/`
2. Add route in `src/server/http_server.rs`
3. Add Lambda handler in `src/lambda/` if needed

### Adding a New Schema Type

1. Define types in `src/schema/types/`
2. Add serialization with `#[derive(Serialize, Deserialize)]`
3. Add TypeScript bindings with `#[cfg_attr(feature = "ts-bindings", derive(TS))]`

### Working with Storage

```rust
// Get schema
let schema = db_ops.get_schema("schema_name").await?;

// Execute query
let results = db_ops.execute_query(&query).await?;

// Perform mutation
let id = db_ops.mutate(mutation).await?;
```

## Important Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Library root, exports public API |
| `src/datafold_node/node.rs` | Main node implementation |
| `src/fold_db_core/fold_db.rs` | Core database logic |
| `src/schema/core.rs` | Schema management |
| `src/logging/mod.rs` | Logging system |
| `Cargo.toml` | Dependencies and features |

## Conventions

- Use `Result<T, SchemaError>` for fallible operations
- Prefer `async fn` over blocking code
- Use `Arc<T>` for shared ownership across async boundaries
- Clone sparingly; prefer references where possible
- Keep handlers thin; business logic in core modules
