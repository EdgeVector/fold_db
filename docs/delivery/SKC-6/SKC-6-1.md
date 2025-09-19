# SKC-6-1 Refactor field processing to use universal key extraction

## Description

Update AtomManager field processing utilities to rely on the universal key helpers instead of heuristic key detection. Ensure Range and HashRange workflows load schema metadata, call `extract_unified_keys()`, and propagate consistent key information when storing molecules and publishing events.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:00:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

- Load schema definitions for incoming `FieldValueSetRequest` operations using existing AtomManager accessors.
- Replace `extract_range_key_from_value` and `extract_hash_key_from_value` heuristics with `extract_unified_keys()` for Range and HashRange fields.
- Normalize shaped payloads with `shape_unified_result()` (or equivalent helper) so downstream consumers receive `{hash, range, fields}` objects.
- Emit precise error messages when universal key extraction fails (missing config, missing data, unsupported dotted paths) without masking failures.
- Maintain backward compatibility for legacy Range schemas by using universal helpers' legacy fallbacks.

## Implementation Plan

1. Introduce a helper in `field_processing.rs` that loads the schema via `manager.db_ops.get_schema()` and calls `extract_unified_keys()` / `shape_unified_result()` for the current request payload.
2. Update `create_range_molecule` and `create_hashrange_molecule` to rely on the new helper outputs instead of direct JSON probing; ensure molecule storage keys use the resolved field names and values.
3. Adjust `create_single_molecule` and `handle_successful_field_value_processing` to include the normalized key snapshot when forming responses and events.
4. Remove obsolete heuristic helpers (`extract_range_key_from_value`, `extract_hash_key_from_value`) and tighten logging to surface universal key extraction failures with actionable context.
5. Ensure `publish_field_value_set_event` populates mutation context hash/range values from the normalized snapshot to keep transform triggers aligned.

## Test Plan

- Add focused unit tests for the new helper to cover Single, Range (legacy + universal), HashRange, and dotted-path key definitions under `tests/unit`.
- Extend integration coverage (e.g., `tests/integration/hashrange_end_to_end_workflow_test.rs`) to assert that events now carry correct universal key metadata.
- Run `cargo test --workspace` to ensure all Rust tests pass.
- Run `cargo clippy --all-targets --all-features` and address lints introduced by the refactor.

## Verification

- Molecule storage keys and mutation events use resolved universal key field names/values across schema types.
- Field processing returns descriptive errors when key configuration is invalid or missing data.
- All existing tests continue to pass alongside the new coverage.

## Files Modified

- `src/fold_db_core/managers/atom/field_processing.rs`
- `src/schema/schema_operations.rs` (if additional helper exports are required)
- `tests/unit/...` (new coverage for field processing)
- `tests/integration/hashrange_end_to_end_workflow_test.rs`

[Back to task list](../tasks.md)
