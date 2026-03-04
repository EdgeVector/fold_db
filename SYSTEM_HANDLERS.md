# System Handlers

## What it does

`src/handlers/system.rs` is the **framework-agnostic handler layer for system read operations**. It provides node identity and health information to both the HTTP server and Lambda without duplicating logic in each runtime.

## Active handlers

| Function | What it does |
|---|---|
| `get_system_status` | Returns health snapshot: status string, uptime seconds, crate version, schema service URL |
| `get_indexing_status` | Returns the current state of the background word-graph indexer via `OperationProcessor` |
| `get_node_private_key` | Returns the node's private key (Base58/Base64, used by the UI for local key management) |
| `get_node_public_key` | Returns the node's public key |

## Architecture

```
HTTP route (server/routes/system.rs)
        │  calls
        ▼
  handlers/system.rs       ← this file
        │  three handlers read directly from FoldNode
        │  get_indexing_status delegates to
        ▼
  OperationProcessor → FoldDB indexing tracker
```

`get_system_status`, `get_node_private_key`, and `get_node_public_key` read directly from `FoldNode` fields — no database lock needed. Only `get_indexing_status` goes through `OperationProcessor`.

## What the original had that the rewrite removes

### Dead code

| Item | Why removed |
|---|---|
| `get_database_config` handler | Never called from any route. `server/routes/system.rs` has its own inline implementation at line 494 |
| `DatabaseConfigResponse` struct | Only used by the dead `get_database_config` handler |
| `ResetDatabaseRequest` struct | `server/routes/system.rs` defines its own local copy; this one is unreachable |
| `ResetDatabaseResponse` struct | Same — routes defines its own; this definition is unreachable |

### Style cleanups (same patterns as prior handler refactors)

| Pattern | Before | After |
|---|---|---|
| 4-line `#[cfg_attr]` ts-bindings attribute | 4 lines per struct | 1 line per struct |
| `/// Response for X` struct doc comments | present | removed |
| Section banner comments (`// ====...====`) | present | removed |
| `match { Ok => ..., Err => Err(...) }` in `get_indexing_status` | 12 lines | `map_err(...)? + Ok(...)` |
| `let private_key = node.get_node_private_key()` intermediate var | 1 line each | inlined |
