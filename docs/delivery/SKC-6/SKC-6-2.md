# SKC-6-2 Normalize mutation requests for universal key payloads

## Description

Ensure mutation pathways supply AtomManager with the data needed for universal key extraction. Standardize the structure of `FieldValueSetRequest` payloads so Range and HashRange mutations include the resolved key field names, raw record fragments, and mutation context derived from the schema configuration.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:05:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

- Derive key field expressions from schema definitions inside `MutationService` using universal key helpers for both Range and HashRange schemas.
- Normalize `FieldValueSetRequest` payloads to include a `fields` object containing the updated field data plus key values, so downstream universal key extraction receives a consistent shape.
- Ensure `MutationContext` passed through the message bus contains the actual hash/range values obtained from the schema helper instead of trusting client payloads.
- Update any direct callers (e.g., transform result storage, HashRange processors) to use the normalized payload builder.
- Maintain backwards compatibility for legacy range mutations by mapping legacy fields into the normalized structure.

## Implementation Plan

1. Add a builder/helper within `MutationService` that constructs normalized request payloads (embedding key values under deterministic keys) for Single, Range, and HashRange mutations.
2. Update `update_range_schema_fields` and `update_hashrange_schema_fields` flows to use the builder, ensuring schema-derived key field names drive payload content and `MutationContext` values.
3. Adjust other producers of `FieldValueSetRequest` (e.g., `transform_manager::hashrange_processor`, `transform_manager::result_storage`) to call the same helper or replicate the normalized format.
4. Extend the `FieldValueSetRequest` struct or constructors if necessary to include a dedicated `fields` map while maintaining serialization compatibility.
5. Audit logs and error handling so mutation requests surface clear diagnostics when schema key metadata cannot be resolved or payload normalization fails.

## Test Plan

- Add unit tests around the new MutationService helper to validate payload structure for Single, Range (legacy + universal), and HashRange schemas.
- Update integration tests that publish `FieldValueSetRequest` events (e.g., `tests/integration/hashrange_end_to_end_workflow_test.rs`) to assert normalized payloads reach AtomManager.
- Run `cargo test --workspace` to validate all tests succeed after the changes.
- Run `cargo clippy --all-targets --all-features` to ensure lint cleanliness.

## Verification

- All mutation pathways emit `FieldValueSetRequest` payloads containing schema-derived key field names and values.
- Mutation context propagated with requests matches the normalized payload and universal key configuration.
- Legacy range mutations continue to function with normalized payload mappings.

## Files Modified

- `src/fold_db_core/services/mutation.rs`
- `src/fold_db_core/infrastructure/message_bus/constructors.rs`
- `src/fold_db_core/transform_manager/hashrange_processor.rs`
- `src/fold_db_core/transform_manager/result_storage.rs`
- `tests/integration/hashrange_end_to_end_workflow_test.rs`
- `tests/unit/...` covering MutationService payload builders

[Back to task list](../tasks.md)
