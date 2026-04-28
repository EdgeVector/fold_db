# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**FoldDB** is a distributed, schema-based database platform with AI-powered data ingestion, designed for personal data sovereignty. It uses Sled as the local embedded database, with optional encrypted cloud sync to Exemem (S3-compatible storage via presigned URLs).

## Build & Test Commands

```bash
# Build
cargo build

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Run Rust tests
cargo test --lib                         # All library tests
cargo test test_name                     # Specific test
cargo test test_name -- --nocapture      # With output

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

## Key Architecture Concepts

### Storage Architecture
- **Always local**: Sled embedded key-value store is the source of truth
- **Optional cloud sync**: When `cloud_sync` is configured in `DatabaseConfig`, the storage stack adds encrypted S3 sync via the Exemem platform

The `KvStore` trait (`src/storage/traits.rs`) provides a unified async interface:
```rust
#[async_trait]
pub trait KvStore: Send + Sync {
    async fn get(&self, key: &[u8]) -> StorageResult<Option<Vec<u8>>>;
    async fn put(&self, key: &[u8], value: Vec<u8>) -> StorageResult<()>;
    // ...
}
```

`DatabaseConfig` is a struct (not an enum):
```rust
pub struct DatabaseConfig {
    pub path: PathBuf,                          // Sled database path
    pub cloud_sync: Option<CloudSyncConfig>,    // Optional Exemem sync
}
```

### Schema System
- Schemas define data structure with fields, permissions, transforms
- States flow: `Pending` -> `Approved` (via backfill process)
- Key types in `src/schema/types/`: `Schema`, `Field`, `Query`, `Mutation`, `Transform`

### Query Sort Order
The `Query` struct supports an optional `sort_order` field (`"asc"` or `"desc"`) that sorts `execute_query_json` results by range key (lexicographic string comparison — works for ISO dates). Defined as `SortOrder` enum in `src/schema/types/operations.rs`. Sorting happens in `query_ops.rs` before rehydration. The LLM agent tool definition instructs the AI to use `sort_order: "desc"` for "most recent" / "latest" queries.

### Handler Pattern
Handlers in `src/handlers/` are framework-agnostic:
```rust
pub async fn handle_query(
    query: Query,
    user_hash: &str,
    node: &FoldNode,
) -> HandlerResult<QueryResponse>
```

### Feature Flags
- `test-utils`: Test utilities for integration tests
- `ts-bindings`: Generates TypeScript type definitions via ts-rs
- `transform-wasm`: WebAssembly transform support for views

## Error Types

- `FoldDbError` (`src/error.rs`): Top-level application error with variants for Schema, Database, Permission, Network, etc.
- `SchemaError` (`src/schema/types/errors.rs`): Domain errors for schema operations
- `StorageError` (`src/storage/error.rs`): Storage backend errors
- `HandlerError` (`src/handlers/response.rs`): API response errors

## Pre-PR Checklist

Before every push, first fetch and rebase on the latest base branch:
```bash
git fetch origin
git rebase origin/<base-branch>   # e.g. origin/main
```

Then run CI checks:
```bash
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

After creating the PR, enable auto-merge:
```bash
gh pr merge --auto <PR_URL>
```

Do NOT pass `--squash`, `--merge`, or `--rebase`. The org-wide merge queue ruleset enforces SQUASH; passing a strategy flag here makes `gh` store an inconsistent `autoMergeRequest.mergeMethod` that the queue silently refuses, stranding the PR. The merge queue rebases the PR onto current main inside its merge group, so "Update branch" never needs to be clicked manually.

**Monitor the PR until it merges — your task is NOT done until the PR is merged.**
Poll CI status (`gh pr view <PR_URL> --json state,statusCheckRollup,mergeStateStatus`) every 30-60 seconds.
- CI failing: read logs (`gh pr checks`), fix the code, push.
- Branch out of date: `git fetch origin && git rebase origin/<base-branch> && git push --force-with-lease`.
- Review comments: fix, push, resolve threads.
- Only done when `state: MERGED`.

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
| `src/fold_node/node.rs` | Main FoldNode - combines DB, security, config |
| `src/fold_db_core/fold_db.rs` | Core database logic |
| `src/fold_db_core/factory.rs` | Creates FoldDB with Sled + optional sync |
| `src/storage/traits.rs` | `KvStore` trait abstraction for storage |
| `src/storage/config.rs` | `DatabaseConfig` struct + `CloudSyncConfig` |
| `src/handlers/mod.rs` | Shared handler layer |
| `src/schema/core.rs` | Schema management |

## Schema Service Environments

The schema service is a separate cloud service deployed at `schema.folddb.com`. Source code lives in [EdgeVector/schema_service](https://github.com/EdgeVector/schema_service); deploy spec in [EdgeVector/schema-infra](https://github.com/EdgeVector/schema-infra). fold_db consumes it as a client.

- **Prod**: `https://axo709qs11.execute-api.us-east-1.amazonaws.com/v1/*` (default)
- **Dev**: `https://y0q3m6vk75.execute-api.us-west-2.amazonaws.com/v1/*` (use `--dev` flag)
- **Local**: `http://127.0.0.1:9102` (use `--local-schema` flag; runs the actix dev binary from the `schema_service` submodule)
