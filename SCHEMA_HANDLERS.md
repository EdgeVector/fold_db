# Schema Handlers

## What it does

`src/handlers/schema.rs` is the **framework-agnostic handler layer for schema management**. It is the single place where schema operations are implemented — the HTTP server and Lambda runtime both call these functions directly, so there is no duplicated routing logic.

## Operations

| Function | What it does |
|---|---|
| `list_schemas` | Returns every schema registered in the database, serialized as JSON with a count |
| `get_schema(name)` | Returns a single schema by name, or `404 Not Found` if it doesn't exist |
| `approve_schema(name)` | Transitions a schema from `Pending` to `Approved`; kicks off a backfill if the schema has transforms; returns the backfill hash |
| `block_schema(name)` | Prevents a schema from accepting queries or mutations |
| `load_schemas` | Scans standard schema directories, registers any schemas not yet in the DB; reports counts of found / loaded / failed |
| `list_schema_keys(name, offset, limit)` | Paginates over the key-value pairs stored under a schema |
| `get_backfill_status(hash)` | Returns the progress of a running or completed backfill, or `404` if the hash is unknown |

## How it fits in the architecture

```
HTTP route / Lambda handler
        │
        ▼
  handlers/schema.rs       ← this file
        │  delegates to
        ▼
  OperationProcessor        (fold_node layer)
        │  calls
        ▼
  FoldDB / SchemaManager    (core database)
```

## Schema state machine

```
  [load_schemas]
       │
       ▼
   Pending  ──[approve_schema]──▶  Approved  ──[block_schema]──▶  Blocked
                                      │
                                      └──[transform present]──▶  backfill starts
```

## What the original code had that the rewrite removes

| Original pattern | Why removed |
|---|---|
| `let processor = OperationProcessor::new(node.clone())` intermediate var | Single-use; inlined into the call chain |
| `match op().await { Ok(x) => Ok(...), Err(e) => Err(HandlerError::Internal(...)) }` | Replaced with `.await.map_err(...)? + Ok(...)` — identical semantics, no nesting |
| `Ok(None) => Err(HandlerError::NotFound(...))` match arm | Replaced with `.ok_or_else(|| HandlerError::NotFound(...))? ` chained after `map_err` |
| 4-line `#[cfg_attr(...)]` ts-bindings attribute | Collapsed to one line |
| `/// Doc comment` on individual struct fields | Removed where they just restate the field name |
| `// Convert to JSON Value` inline comments | Removed — the code is self-evident |
