# MutationService Reference

The `MutationService` assembles normalized mutation payloads that travel across the message bus to `AtomManager`. It is the
single entry point for constructing `FieldValueSetRequest` events and guarantees a consistent `{ hash, range, fields }` shape
for every schema type.

## Normalized Request Builder

The `MutationService::normalized_field_value_request` helper loads schema metadata, resolves universal key values, and returns
a `NormalizedFieldValueRequest` wrapper:

```rust
let builder = MutationService::new(message_bus.clone());
let schema = schema_store.load("BlogPostWordIndex");
let request = builder
    .normalized_field_value_request(
        &schema,
        "content",
        &Value::String("AI updates".into()),
        Some(&Value::String("technology".into())),
        Some(&Value::String("2025-01-15".into())),
        Some("mutation-uuid"),
    )?;
```

`NormalizedFieldValueRequest` exposes two fields:

- `request`: the serialized `FieldValueSetRequest` event that is ready to publish.
- `context`: a lightweight `NormalizedFieldContext` containing the resolved hash, range, and payload field map for downstream
  consumers that do not need to deserialize the request body again.

### Context Normalization Rules

* Empty strings are converted to `None` in the context object so callers can distinguish "missing" from "provided but empty".
* Field names are sorted consistently before serialization to avoid nondeterministic payload signatures.
* Mutation hashes are threaded through the `MutationContext` when provided so incremental processors can link follow-up events.

## FieldValueSetRequest Payload Structure

`FieldValueSetRequest::from_normalized_parts` takes a `NormalizedRequestParts` struct and emits the transport payload consumed by
`AtomManager`:

```json
{
  "correlation_id": "uuid",
  "schema_name": "BlogPostWordIndex",
  "field_name": "content",
  "value": {
    "hash": "technology",
    "range": "2025-01-15",
    "fields": {
      "content": "AI updates",
      "publish_date": "2025-01-15",
      "word": "technology"
    }
  },
  "source_pub_key": "web-ui",
  "mutation_context": {
    "hash_key": "technology",
    "range_key": "2025-01-15",
    "mutation_hash": "mutation-uuid",
    "incremental": true
  }
}
```

Important guarantees:

1. `value.hash` and `value.range` are always present but may be empty strings when a schema type does not supply the key.
2. `value.fields` contains the normalized field map produced by `shape_unified_result`, ensuring dotted-path extractions are
   flattened into the event payload.
3. `mutation_context` is populated whenever either key is present or a `mutation_hash` was provided, supplying AtomManager and
   downstream processors with consistent incremental metadata.

## Failure Modes

The builder surfaces schema-driven validation errors rather than constructing partial payloads:

- Missing schema definitions raise `SchemaError::InvalidData` before any request is published.
- If a HashRange schema omits either hash or range values, request construction fails with descriptive error messages and a
  structured infrastructure log entry.
- Dotted-path key extraction uses `extract_unified_keys` and bubbles up the precise field that failed to resolve, helping callers
  correct payloads quickly.

## Workflow Integration

1. `MutationService` publishes the normalized `FieldValueSetRequest`.
2. `AtomManager` receives the event, invokes `resolve_universal_keys`, and stores a `KeySnapshot` alongside the processed atom.
3. `FieldValueSetResponse` echoes the normalized snapshot so callers can correlate stored atoms with universal key metadata.

Refer to the [Field Processing reference](./field_processing.md) for AtomManager behavior and to the
[Universal Key Migration Guide](../../universal-key-migration-guide.md#field-processing-and-mutation-workflow) for an operational
overview.
