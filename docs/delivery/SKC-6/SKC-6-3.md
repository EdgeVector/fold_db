# SKC-6-3 Add universal key field processing test coverage

## Description

Expand automated testing to verify field processing utilities operate correctly with universal key configuration across schema types. Cover Range, HashRange, Single schemas, dotted-path keys, and error scenarios to guard against regressions introduced by the refactor.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:10:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements

- Create unit tests targeting the new field processing helper to validate key extraction and shaped payloads for Single, Range, and HashRange schemas.
- Add regression tests for dotted-path key expressions and missing key data error handling.
- Extend integration tests (including end-to-end hash range workflow) to assert stored molecules/events contain correct universal key metadata.
- Ensure new tests cover both success and failure paths for universal key processing.
- Document new test cases inline with comments for future maintainers.

## Implementation Plan

1. Add Rust unit tests under `tests/unit/fold_db_core/atom_manager/` (or equivalent) focused on the normalized field processing helper.
2. Update `tests/integration/hashrange_end_to_end_workflow_test.rs` and other relevant integration suites to validate universal key metadata propagation.
3. Introduce fixture builders for schemas with dotted-path keys to simplify repetitive setup in tests.
4. Ensure existing legacy range key tests remain intact or are ported to new helpers for backward compatibility coverage.
5. Add negative tests asserting descriptive errors when key configuration is missing or payloads omit required key data.

## Test Plan

- Run `cargo test --workspace` to execute all new and existing Rust tests.
- Run `cargo clippy --all-targets --all-features` to confirm lint cleanliness for the test additions.

## Verification

- New tests fail without the universal key refactor and pass after implementation.
- Integration tests confirm FieldValueSet events and stored molecules carry the expected universal key metadata.
- Negative scenarios produce the documented error messages.

## Files Modified

- `tests/integration/hashrange_end_to_end_workflow_test.rs`
- `tests/unit/...` (new unit tests for AtomManager field processing)
- `tests/unit/unified_key_extraction_tests.rs` (extended cases for dotted paths / errors)
- Supporting test fixtures or helpers under `tests/`

[Back to task list](../tasks.md)
