# Field Processing Reference

`src/fold_db_core/managers/atom/field_processing.rs` owns the ingestion path for `FieldValueSetRequest` events. The module was refactored in SKC-6 to rely entirely on schema-driven universal key helpers, eliminating heuristic fallbacks and ensuring deterministic payloads for downstream consumers.

## Key Components

- **`ResolvedAtomKeys`** – Lightweight struct returned by `resolve_universal_keys` containing the resolved hash, range, and normalized field map.
- **`resolve_universal_keys`** – Loads the schema, invokes `extract_unified_keys` and `shape_unified_result`, and converts the shaped JSON into a `KeySnapshot`.
- **`handle_fieldvalueset_request`** – Entry point that creates atoms, attaches molecules, and publishes `FieldValueSetResponse` events using the normalized metadata.
- **`KeySnapshot`** – Serialized representation published with `FieldValueSet` events so downstream listeners can reuse the normalized keys without recalculating them.

## Workflow

1. `MutationService` publishes a normalized `FieldValueSetRequest` (see [MutationService reference](./mutation_service.md)).
2. `AtomManager` receives the request and calls `resolve_universal_keys` using the stored schema metadata.
3. The helper persists the resulting `KeySnapshot` alongside the atom and returns `ResolvedAtomKeys` to the caller.
4. `handle_fieldvalueset_request` uses the normalized context to populate molecules and emit `FieldValueSet` events without touching schema-specific logic.

This workflow mirrors the [Universal Key Processing Workflow](../../guides/operations/universal-key-migration-guide.md#universal-key-processing-workflow) and guarantees that every consumer interacts with `{ hash, range, fields }` data in the same format.

## Dotted-Path Resolution

Universal key expressions support dotted paths (e.g., `metadata.partition.hash`). `resolve_universal_keys` delegates to `shape_unified_result`, which walks the payload using serde to produce a flattened map. When a dotted path fails to resolve, the helper returns `SchemaError::InvalidData` and includes the offending expression in the error message.

Example:

```json
{
  "metadata": {
    "partition": {
      "hash": "region-1",
      "range": "2025-01-20"
    }
  },
  "content": "..."
}
```

- `key.hash_field = "metadata.partition.hash"`
- `key.range_field = "metadata.partition.range"`

The resolved snapshot contains:

```json
{
  "hash": "region-1",
  "range": "2025-01-20",
  "fields": {
    "content": "...",
    "metadata.partition.hash": "region-1",
    "metadata.partition.range": "2025-01-20"
  }
}
```

If any segment is missing, AtomManager logs the failure and returns a `FieldValueSetResponse` with `success = false` instead of fabricating defaults.

## Error Handling

Field processing code surfaces failures via `SchemaError` and never performs silent fallbacks. Common scenarios include:

- **Missing schema** – `SchemaError::InvalidData("Schema 'X' not found")`; verify the schema is approved and loaded before issuing requests.
- **Missing key values** – Occurs when mutations omit required hash or range fields. The response includes a descriptive error message and no atoms are persisted.
- **Dotted path mismatch** – Raised when the payload structure does not match the configured key expressions. Compare the payload with the example above.

All errors are logged with emoji-prefixed messages to aid troubleshooting (e.g., `❌ Failed to extract keys`).

## Troubleshooting Tips

1. Enable debug logging to review the `ResolvedAtomKeys` summary emitted for each request.
2. Validate schemas with dotted paths using unit tests in `tests/unit/field_processing/`.
3. Inspect `FieldValueSetResponse` payloads in the message bus to confirm the `KeySnapshot` fields match expectations.
4. Cross-reference the troubleshooting table in the [Universal Key Migration Guide](../../guides/operations/universal-key-migration-guide.md#troubleshooting) for system-wide symptoms.

## Related Resources

- [MutationService reference](./mutation_service.md)
- [Universal Key Migration Guide](../../guides/operations/universal-key-migration-guide.md)
