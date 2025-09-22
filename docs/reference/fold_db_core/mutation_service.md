# MutationService Reference

The `MutationService` coordinates schema-driven mutations and is the single entry point for constructing normalized `FieldValueSetRequest` messages. This document explains how the service resolves universal key metadata, shapes payloads, and surfaces troubleshooting guidance introduced in SKC-6.

## Responsibilities

- Load schema definitions and validate that universal key configuration is present when required.
- Call the shared helpers (`extract_unified_keys`, `shape_unified_result`) to gather `{ hash, range, fields }` metadata.
- Construct `FieldValueSetRequest` payloads via `FieldValueSetRequest::from_normalized_parts`, ensuring sorted field maps and consistent key normalization.
- Emit a `NormalizedFieldContext` summary so callers can log the resolved key state without parsing JSON.
- Publish normalized requests on the message bus so AtomManager can persist a `KeySnapshot` without recomputing keys.

## Normalized Field Value Builder

`MutationService::normalized_field_value_request` is the preferred helper for building mutation payloads. The method returns both the serialized request and the normalized context for diagnostic use:

```rust
let normalized = mutation_service.normalized_field_value_request(
    &schema,
    "content",
    &Value::String("hello world".into()),
    None,              // optional hash value override
    None,              // optional range value override
    Some("trace-123"), // mutation hash propagated to downstream consumers
)?;

message_bus.publish(normalized.request.clone())?;
log::debug!("normalized context = {:?}", normalized.context);
```

Internally the builder performs three steps:

1. Copies the field value into a map while preserving the original JSON structure.
2. Resolves hash and range key values using schema metadata. Provided overrides are respected; otherwise the helper falls back to values from the payload.
3. Invokes `FieldValueSetRequest::from_normalized_parts`, which sorts field names and stores `hash` and `range` as `None` when blank.

### Payload Contract

The normalized request always serializes to the following structure:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `schema_name` | `String` | Name of the schema being mutated. |
| `field_name` | `String` | The field that triggered the mutation. |
| `value.fields` | `Map<String, Value>` | Sorted map containing the mutated field plus any additional context emitted by `shape_unified_result`. |
| `value.hash` | `String` | Hash key rendered as a string; empty string indicates absence. |
| `value.range` | `String` | Range key rendered as a string; empty string indicates absence. |
| `mutation_context.hash_key` | `Option<String>` | Populated when a hash key exists. |
| `mutation_context.range_key` | `Option<String>` | Populated when a range key exists. |
| `mutation_context.mutation_hash` | `Option<String>` | Optional trace identifier forwarded by the caller. |
| `mutation_context.incremental` | `bool` | Indicates whether the request targets an incremental update. |

Downstream services must treat empty strings for `hash` or `range` as "key not provided". The helper normalizes whitespace-only strings to `None` before serialization so blank values cannot leak into analytics or storage layers.

## Error Handling

`MutationService` surfaces schema-driven failures using `SchemaError` variants:

- `SchemaError::InvalidData` indicates missing key values (HashRange schemas) or payloads that cannot be normalized.
- `SchemaError::MissingKeyConfiguration` is raised when the schema omits a required key configuration.
- Any failure to load the schema propagates immediately; callers should surface the error to clients without retrying.

Logs emitted by the service include the `NormalizedFieldContext` summary in the format `hash:present|missing` to simplify diagnostics. See the troubleshooting section in the [Universal Key Migration Guide](../../guides/operations/universal-key-migration-guide.md#troubleshooting) for common remediation steps.

## Integration Points

- **AtomManager**: Consumes normalized requests, persists atoms, and stores a `KeySnapshot` that mirrors the `{ hash, range, fields }` values. Details are covered in the [field processing reference](./field_processing.md).
- **Message bus constructors**: `FieldValueSetRequest::from_normalized_parts` lives in `src/fold_db_core/infrastructure/message_bus/constructors.rs` and enforces the normalized layout used across the system.
- **Transform manager**: Downstream processors read `mutation_context` instead of parsing the payload manually, ensuring consistent key usage.

## Troubleshooting Checklist

1. Confirm the schema's key configuration matches the values provided in the mutation payload.
2. Enable debug logging for `MutationService` to review the normalized context summary.
3. Use unit tests under `tests/unit/mutation/` to reproduce failures in isolation.
4. Reference the dotted-path guidance in the field processing documentation if key expressions navigate nested JSON.

For architectural context, review the [Universal Key Processing Workflow](../../guides/operations/universal-key-migration-guide.md#universal-key-processing-workflow).
