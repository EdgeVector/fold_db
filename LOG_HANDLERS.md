# Log Handlers

## What it does

`src/handlers/logs.rs` is the **framework-agnostic handler layer for log management**. It exposes five operations that are shared between the HTTP server and the Lambda runtime — neither caller needs to know where the data comes from.

## Operations

| Function | What it does |
|---|---|
| `list_logs(since, user_hash, node)` | Returns up to 1,000 log entries, optionally filtered by timestamp |
| `get_log_config(user_hash, node)` | Returns the current `LogConfig` (levels, feature map) |
| `get_log_features(user_hash, node)` | Returns the per-feature log levels plus the list of valid level strings |
| `update_log_feature_level(feature, level, user_hash, node)` | Sets one feature's log level at runtime (e.g. `"ingestion"` → `"DEBUG"`) |
| `reload_log_config(config_path, user_hash, node)` | Reloads `LogConfig` from a file on disk |

## How it fits in the architecture

```
HTTP route / Lambda handler
        │
        ▼
  handlers/logs.rs          ← this file
        │  delegates to
        ▼
  OperationProcessor         (fold_node layer)
        │  calls
        ▼
  LoggingSystem              (global singleton backed by OnceLock)
```

Every handler follows the same three-step pattern:
1. Create an `OperationProcessor` from the `FoldNode`
2. Call the appropriate `LoggingSystem` method through the processor
3. Wrap the result in `ApiResponse::success_with_user` or propagate a `HandlerError::Internal`

## Response types

- **`LogListResponse`** — `{ logs: Value, count: usize, timestamp: u64 }`
- **`LogConfigResponse`** — `{ config: Value }` (the full `LogConfig` serialized)
- **`LogFeaturesResponse`** — `{ features: Value, available_levels: Vec<String> }`
- Simple mutations use the shared **`SuccessResponse`** — `{ success: bool, message: Option<String> }`

## What the original code had that the rewrite removes

| Original pattern | Why removed |
|---|---|
| `if let Some(x) = ... { ok } else { hardcoded fallback }` | Fallbacks hide the fact that `LoggingSystem` isn't initialized — violates "no fallbacks" rule |
| `match result { Ok(_) => success, Err(e) => Err(...) }` | Replaced with `map_err(...)?` + explicit `Ok(...)` — same semantics, less nesting |
| Hardcoded feature-name list in the `None` branch of `get_log_features` | Removed with the fallback; the list of valid features comes from the live config |
| `vec!["TRACE"..., "ERROR".to_string()]` repeated inline | Replaced with a `const` array |
| `let processor = OperationProcessor::new(node.clone())` intermediate variable | Inlined where the processor is used only once |
| Section banner comments (`// ====...====`) | Removed — the function names and types are self-documenting |
