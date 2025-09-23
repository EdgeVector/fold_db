# NTS-1-4 Add comprehensive unit tests

[Back to task list](./tasks.md)

## Description
Develop exhaustive unit test coverage for the new native transform primitives so that all validation rules and conversion paths are locked down before downstream integration work begins. The focus is on exercising success and failure cases for `FieldValue`, `FieldType`, `FieldDefinition`, and `TransformSpec`, ensuring native behaviour is stable and regressions are caught immediately.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-24 09:00:00 | Status Change | N/A | Proposed | Task file created with initial testing scope outlined | AI_Agent |
| 2025-09-24 09:05:00 | Status Change | Proposed | InProgress | Began expanding unit tests for native transform primitives | AI_Agent |
| 2025-09-24 11:45:00 | Status Change | InProgress | Review | Submitted expanded unit tests for review with all checks passing | AI_Agent |

## Requirements
- Cover all `FieldValue` conversions, including JSON fallbacks for non-finite numbers.
- Exercise `FieldDefinition` validation for every documented error variant and confirm default resolution semantics.
- Validate `TransformSpec` success paths and ensure each error case surfaces the correct typed error.
- Keep unit tests focused and deterministic, avoiding reliance on integration scaffolding.
- Update architectural documentation to capture the new test coverage guarantee.

## Implementation Plan
1. Extend `tests/unit/native_types_tests.rs` with scenarios for JSON number fallbacks and null-only arrays to ensure `FieldValue` inference remains stable.
2. Add `NativeFieldDefinition` tests for invalid starting characters, over-length names, whitespace handling, and required-field default resolution.
3. Expand `NativeTransformSpec` tests to hit every validation error branch, including empty transforms, duplicate inputs, invalid mappings, reducer mistakes, and unknown references.
4. Update `docs/project_logic.md` with a new rule documenting the comprehensive unit test contract for native transform primitives.
5. Synchronize task tracking metadata and execute formatting, linting, Rust workspace tests, and the required frontend Vitest suite.

## Verification
- Added tests fail against the old implementation and pass with the current code.
- Every validation error variant in `FieldDefinitionError` and `TransformSpecError` is exercised by at least one unit test.
- JSON conversion helpers are covered for edge cases such as `NaN` fallbacks and null-only arrays.
- Documentation and task tracking accurately reflect the new testing mandate.
- All repository checks (`cargo fmt`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features`, and frontend `npm test`) pass successfully.

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/project_logic.md`
- `tests/unit/native_field_definition_tests.rs`
- `tests/unit/native_transform_spec_tests.rs`
- `tests/unit/native_types_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm install)`
- `(cd src/datafold_node/static-react && npm test)`
