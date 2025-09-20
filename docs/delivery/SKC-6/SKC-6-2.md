# SKC-6-2 Refactor Single and Range molecule creation to use universal key snapshot

## Description
Adopt the universal key snapshot helper within the Single and Range field processing flows so molecule storage and responses rely on schema-driven key data rather than heuristic JSON extraction. This task focuses on the standard Single and Range pathways and leaves HashRange-specific logic to follow-up work.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:05:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-23 09:10:00 | Scope Refined | Proposed | Proposed | Limited task to Single + Range adoption of the new helper | ai-agent |

## Requirements
- Invoke the new universal key helper inside `create_single_molecule` and `create_range_molecule` so each function receives resolved key names and values before persisting data.
- Ensure Range molecule UUIDs and storage keys use the schema-defined key names/values rather than hardcoded `range_key` fields.
- Update responses returned by `handle_successful_field_value_processing` for Single and Range mutations to include the normalized key snapshot (hash/range/fields) for downstream consumers.
- Remove direct calls to `extract_range_key_from_value` or other heuristic JSON access within the Single/Range code paths.
- Preserve backward compatibility for legacy Range schemas by relying on the universal helper's legacy fallback logic; no manual branching should remain in these functions.

## Implementation Plan
1. Thread the `ResolvedAtomKeys` output from the helper into `create_molecule_for_field`, allowing Single and Range branches to share the resolved data.
2. Refactor `create_single_molecule` to persist the normalized `fields` payload and return the helper output alongside the molecule UUID as needed by callers.
3. Refactor `create_range_molecule` to build molecule IDs, persistence keys, and cache entries using the resolved range key value while storing the normalized fields map.
4. Extend `handle_successful_field_value_processing` to attach the helper snapshot to the `FieldValueSetResponse` and to any instrumentation emitted for Single/Range paths.

## Test Plan
- Update or add unit tests that cover Single and Range molecule creation, ensuring the returned responses carry schema-derived key names and values.
- Extend integration coverage (e.g., `tests/integration/hashrange_end_to_end_workflow_test.rs` or a new Single/Range-focused test) to confirm Range molecules are written using the normalized key snapshot.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` after implementing the refactor.

## Verification
- Single and Range molecule creation no longer reference heuristic `extract_*` helpers and rely exclusively on the universal snapshot.
- Field value responses expose the normalized key data for Single and Range mutations.
- Legacy Range schemas continue to function using the universal helper's fallback logic.

## Files Modified
- `src/fold_db_core/managers/atom/field_processing.rs`
- `tests/integration/range_field_processing_flow_test.rs`
- `tests/unit/field_processing/single_and_range_tests.rs`

[Back to task list](../tasks.md)
