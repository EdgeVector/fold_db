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

## Bump cascade (downstream)

Downstream consumers pull this repo's main HEAD on a **2-hour schedule**: `EdgeVector/schema_service`'s `bump-fold-db.yml` (cron) bumps its fold_db rev pin, then `EdgeVector/fold_db_node`'s `bump-schema-service.yml` (cron) picks up schema_service's tip plus the matching fold_db rev. End-to-end cascade lag: up to 2h schema_service hop + up to 2h fold_db_node hop. The cascade used to be dispatch-driven from this repo via `notify-downstream.yml`, but per-merge bumps superseded each other in downstream merge queues — switched to scheduled polling 2026-05-01.

If you're shipping a breaking fold_db change that requires consumer Rust updates alongside the rev bump, disable the relevant `bump-*.yml` workflow in `EdgeVector/schema_service` and `EdgeVector/fold_db_node` Actions before merging here, do the manual consumer PRs, re-enable. Force-flush an immediate bump cycle (rather than waiting up to 2h) via `gh workflow run bump-fold-db.yml -R EdgeVector/schema_service` and `gh workflow run bump-schema-service.yml -R EdgeVector/fold_db_node` after this lands.

## AI Provider Configuration

AI ingestion runs against either **Ollama** (local, default — auto-detected at `http://127.0.0.1:11434`) or **Anthropic** (cloud, direct calls to `https://api.anthropic.com` per [`llm_registry/models.rs`](crates/core/src/llm_registry/models.rs)).

The provider choice and Anthropic API key live in the node config store (`crates/core/src/storage/node_config_store.rs`) and are configured via the **Settings → AI Provider** tab in the web UI — **not** via environment variables. There is no `FOLD_*_API_KEY` env var for AI providers.

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

Don't bother passing a strategy flag. The org-wide merge queue ruleset enforces SQUASH at land time, but GitHub's API silently overrides `autoMergeRequest.mergeMethod = MERGE` on any merge-queue branch — verified that even `enablePullRequestAutoMerge(mergeMethod: SQUASH)` via GraphQL returns `MERGE`. There's no client-side way to set anything else. The queue still squashes when it actually lands the PR (commits arrive as 1-parent squash), so this discrepancy is cosmetic. The "merge strategy is set by the merge queue" warning gh prints if you do pass a flag is honest — gh is telling you it ignored your flag.

After CI goes green there's a `min_entries_to_merge_wait_minutes: 5` delay before the queue admits the PR — that wait is normal, not a strand. The queue also rebases the PR onto current main inside its merge group, so "Update branch" never needs to be clicked manually.

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

## Observability conventions

When you touch any `tracing::*!`, `tokio::spawn`, `reqwest::Client::new()`, or sensitive-field log site, the conventions live at `gbrain get concepts/observability-conventions`. Long-form: `exemem-workspace/docs/observability/migration-guide.md`. CI-enforced rules:

- **Structured fields** — `tracing::info!(field = %value, "msg")`. Positional-arg interpolation gets a warn-only nudge.
- **Redaction** — `password / token / api_key / secret / auth_token / email / phone / ssn` MUST go through `redact!()` or `redact_id!()`. Override per-line: `// lint:redaction-ok <reason>`.
- **`tokio::spawn`** — chain `.instrument(Span::current())` or `.in_current_span()`. Bare spawn fails CI. Override: `// lint:spawn-bare-ok <reason>`.
- **Outbound `reqwest`** — every `Client::new() / ::builder() / ::default()` site needs a comment within 3 preceding lines: `propagate / loopback / skip-s3 / skip-3p`. Wrap propagating requests with `observability::propagation::inject_w3c`.

The cloud-stack cleanup (2026-04-28) removed OTLP / Honeycomb / SpanMetrics. Do NOT add `opentelemetry-otlp` / `opentelemetry-proto` deps or set `OBS_OTLP_ENDPOINT`. Sentry stays (off-by-default; activates if `OBS_SENTRY_DSN` is set).

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

The canonical URL registry lives in [EdgeVector/fold_db_node](https://github.com/EdgeVector/fold_db_node) at `environments.json` (single source of truth across the workspace; `scripts/lint-no-hardcoded-urls.sh` enforces it). Resolve current URLs with `~/code/edgevector/fold_db_node/scripts/get-env-url.sh <dev|prod> schema_service` — do not hardcode them in this repo.

- **Prod**: us-east-1 API Gateway (release builds default to this)
- **Dev**: us-west-2 API Gateway (`--dev` flag, or debug builds)
- **Local**: `http://127.0.0.1:9102` (`--local-schema` flag; runs the actix dev binary from the `schema_service` submodule)
