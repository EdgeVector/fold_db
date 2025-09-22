# Field Processing Reference

`AtomManager`'s field processing pipeline consumes `FieldValueSetRequest` events published by `MutationService` and persists the
resulting atoms, molecules, and normalized key metadata.

## Universal Key Resolution

The [`resolve_universal_keys`](../../../src/fold_db_core/managers/atom/field_processing.rs) helper is the first step for every
request. It performs three responsibilities:

1. Loads the schema definition from storage, returning a `SchemaError::InvalidData` if the schema cannot be found.
2. Uses `extract_unified_keys` to resolve hash and range values from the request payload for any schema type (Single, Range,
   HashRange), including dotted-path lookups.
3. Calls `shape_unified_result` to produce the normalized `{ hash, range, fields }` map stored in a `ResolvedAtomKeys` snapshot.

The snapshot is later serialized into the `KeySnapshot` that rides on the `FieldValueSetResponse` message.

## FieldValueSetRequest Handling Flow

1. `handle_fieldvalueset_request` receives the normalized request from the message bus and records statistics.
2. `create_atom_for_field_value` writes the new atom and returns its UUID.
3. `create_molecule_for_field` stores the derived molecule, reusing the `ResolvedAtomKeys` snapshot to ensure molecules and
   downstream events share the same key metadata.
4. `handle_successful_field_value_processing` publishes a `FieldValueSetResponse` containing the molecule UUID and the
   `KeySnapshot` built from the resolved keys.
5. Any failure along the way short-circuits and emits a detailed error response; no fallback heuristics attempt to guess key
   values.

### KeySnapshot Example

A successful response includes the normalized snapshot:

```json
{
  "correlation_id": "uuid",
  "success": true,
  "molecule_uuid": "atom-uuid",
  "key_snapshot": {
    "hash": "technology",
    "range": "2025-01-15",
    "fields": {
      "word": "technology",
      "publish_date": "2025-01-15",
      "content": "AI updates"
    }
  }
}
```

Consumers can rely on the snapshot to understand exactly what key metadata was persisted without rehydrating the atom payload.

## Troubleshooting Signals

* Missing schema or key configuration errors bubble up from `resolve_universal_keys` and are logged with the schema name and
  field that failed validation.
* Dotted-path extraction failures annotate the missing path segment, indicating whether the hash or range lookup failed.
* Inconsistent payloads (e.g., mismatched hash/range between molecule creation and response publishing) cannot occur because the
  same `ResolvedAtomKeys` snapshot is reused across all steps.

See the [Universal Key Migration Guide](../../universal-key-migration-guide.md#field-processing-and-mutation-workflow) for an
operational overview and escalation steps.
