# NTS-1-4 Add comprehensive unit tests

[Back to task list](./tasks.md)

## Description
Strengthen confidence in the native transform data model by adding thorough unit
coverage for the newly introduced types. This task focuses on verifying
identifier validation, default handling, and all branches of `TransformSpec`
validation so future execution logic can rely on predictable error reporting.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 19:21:00 | Status Change | N/A | Proposed | Documented test coverage expansion scope | AI_Agent |
| 2025-09-22 19:21:30 | Status Change | Proposed | In Progress | Began implementing additional native transform unit tests | AI_Agent |

## Requirements
- Exercise identifier validation failures across `FieldDefinition` to cover
  whitespace, starting character, and maximum length guards.
- Verify `FieldDefinition::effective_default` behaviour for both required and
  optional fields.
- Extend `TransformSpec` unit tests to cover name validation, duplicate inputs,
  input/output definition errors, and every map/filter/reduce/chain error
  variant that can be constructed without integration context.
- Maintain clarity and reuse in tests by introducing focused helpers where they
  reduce duplication.
- Keep existing tests passing and avoid altering production behaviour.

## Implementation Plan
1. Update `docs/delivery/NTS-1/tasks.md` to mark the task as in progress and
   document it here.
2. Add new negative-path tests to `tests/unit/native_field_definition_tests.rs`
   covering identifier and default validation edge cases.
3. Expand `tests/unit/native_transform_spec_tests.rs` with additional scenarios
   for map, filter, reduce, and chain validation errors, reusing helpers to keep
   fixtures concise.
4. Run `cargo fmt` to maintain consistent formatting for the updated test
   modules.
5. Execute the Rust and frontend test suites required by repository policy.

## Verification
- All new tests compile and assert the expected error variants.
- Existing native transform unit tests continue to pass.
- Repository formatting, linting, and required test suites succeed (allowing for
  documented pre-existing frontend issues).

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/delivery/NTS-1/NTS-1-4.md`
- `tests/unit/native_field_definition_tests.rs`
- `tests/unit/native_transform_spec_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm install && npm test -- --watch=false)`
