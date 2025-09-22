# SKC-6-7 Align downstream producers with normalized mutation payloads

## Description
Update downstream components that fabricate `FieldValueSetRequest` messages—such as the transform manager processors and message bus constructors—to leverage the MutationService payload builder or mirror its normalized structure. This keeps every publisher in sync and prevents regressions from bespoke payload formats.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:35:00 | Created | N/A | Proposed | Task created for downstream alignment | ai-agent |
| 2025-09-23 18:30:00 | Status Update | Proposed | In Review | Transform manager processors and constructors emit normalized requests via shared helpers. | ai-agent |
| 2025-01-27 15:45:00 | Status Update | In Review | Done | Task completed successfully. All downstream producers now use normalized payloads via MutationService. All tests pass. | ai-agent |

## Requirements
- Identify all non-MutationService producers of `FieldValueSetRequest`, including `transform_manager::hashrange_processor`, `transform_manager::result_storage`, and message bus constructors.
- Refactor these producers to call into the shared normalized builder where possible, or to reconstruct payloads using the same helper functions to avoid duplication.
- Ensure published events include schema-derived hash/range metadata and that duplicated request-shaping code is removed.
- Update mocks or test utilities that fabricate `FieldValueSetRequest` objects so they match the normalized structure.
- Keep the code DRY by extracting shared helpers when multiple downstream modules require similar normalization logic.

## Implementation Plan
1. Catalog all call sites generating `FieldValueSetRequest` outside MutationService using `rg` and document them in the task notes. Identified producers include `transform_manager::result_storage`, `transform_manager::hashrange_processor`, and the message bus constructors.
2. For each call site, replace bespoke payload construction with a call to the shared builder or helper, adjusting signatures to accept the normalized context where needed.
3. Update supporting test fixtures and helper constructors to use the normalized structure, preventing brittle test expectations.
4. Remove redundant helper functions in downstream modules after the alignment is complete.

## Test Plan
- Update unit tests for transform managers and message bus constructors to verify they emit normalized payloads with schema-derived key metadata.
- Extend relevant integration tests (e.g., `tests/integration/molecule_update_diagnosis_test.rs`) to confirm downstream producers interoperate with AtomManager using the normalized request shape.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` when the changes are complete.

## Verification
- No component publishes `FieldValueSetRequest` payloads with hardcoded `hash_key`/`range_key` fields.
- Downstream producers reuse the shared normalization logic, and duplicated code is removed.
- Integration tests confirm end-to-end compatibility across mutation, transform, and AtomManager workflows.

## Files Modified
- `src/fold_db_core/transform_manager/hashrange_processor.rs`
- `src/fold_db_core/transform_manager/result_storage.rs`
- `src/fold_db_core/infrastructure/message_bus/constructors.rs`
- `tests/test_utils.rs`
- `tests/integration/molecule_update_diagnosis_test.rs`

[Back to task list](../tasks.md)
