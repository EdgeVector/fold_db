# SKC-6-6 Adopt normalized payload builder in mutation workflows

## Description
Replace direct `FieldValueSetRequest` construction within MutationService workflows with the normalized builder introduced earlier. This covers Single, Range, and HashRange update paths and ensures every published request includes schema-derived key metadata and normalized payload structures.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:30:00 | Created | N/A | Proposed | Task created to roll out the new payload builder | ai-agent |
| 2025-09-23 16:45:00 | Status Update | Proposed | Done | Mutation workflows now emit builder-normalized payloads with updated range/Single integration tests. | ai-agent |

## Requirements
- Update `update_range_schema_fields`, `update_hashrange_schema_fields`, and any other MutationService entry points that emit `FieldValueSetRequest` so they call the normalized builder.
- Ensure the returned context struct from the builder is used to populate `MutationContext` hash/range values instead of trusting caller-provided payloads.
- Standardize the shape of the emitted request payloads (`{ fields, hash, range }`), removing ad-hoc JSON assembly and duplicated logic across mutation paths.
- Maintain backward compatibility for legacy Range schemas by relying on the builder's fallback handling; no extra branching should be introduced in the workflows.
- Update logging/events emitted by MutationService to reflect the normalized payload structure without exposing sensitive field values.

## Implementation Plan
1. Replace inline `FieldValueSetRequest::new` and `with_context` invocations with calls to the normalized builder, threading through the returned context data.
2. Update `MutationContext` population so hash/range values come from the normalized context rather than the raw client payload.
3. Remove redundant payload-building code paths once the builder is in place, keeping shared helper usage DRY.
4. Review and update logging to ensure new payload structure is reflected succinctly, removing any println diagnostics.

## Test Plan
- Update existing mutation workflow unit tests to expect the normalized payload structure and context propagation.
- Add or extend integration tests covering Range and HashRange mutation flows to assert that AtomManager receives schema-derived key data.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` to validate the refactor.

## Verification
- All mutation workflows publish normalized payloads with schema-derived hash/range values.
- Mutation context objects reflect the normalized keys, eliminating reliance on client-provided fields.
- No duplicated payload construction logic remains outside the builder.

## Files Modified
- `src/fold_db_core/services/mutation.rs`
- `tests/integration/complete_mutation_query_flow_test.rs`
- `tests/integration/mutation_range_workflow_test.rs`

[Back to task list](../tasks.md)
