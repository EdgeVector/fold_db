# NTS-1-2 Implement FieldDefinition struct with validation

[Back to task list](./tasks.md)

## Description
Introduce native schema field definitions that pair the new `FieldValue`/`FieldType` enums with validation and defaulting logic. This struct will provide the foundation for building transform specifications without relying on loosely typed JSON metadata.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 19:16:00 | Status Change | N/A | Proposed | Task file created with implementation outline | AI_Agent |
| 2025-09-22 19:16:30 | Status Change | Proposed | In Progress | Began implementing native field definition struct and validation | AI_Agent |
| 2025-09-23 09:30:00 | Status Change | In Progress | Review | Implementation complete with comprehensive tests | AI_Agent |
| 2025-09-23 09:35:00 | Status Change | Review | Done | Task completed and approved - all tests passing | AI_Agent |

## Requirements
- Define a `FieldDefinition` struct in the native transform module with `name`, `field_type`, `required`, and `default_value` fields.
- Provide validation helpers that return typed errors for invalid field names and mismatched default values.
- Support deriving sensible default values based on the declared field type so optional fields can be auto-populated.
- Re-export the struct (and its error type) through `transform::` for downstream consumers.
- Add comprehensive unit tests covering validation success/failure and default generation behavior.
- Update `docs/project_logic.md` to capture the new native field definition rule.

## Implementation Plan
1. Create `src/transform/native/field_definition.rs` containing the struct, error enum, validation helpers, and default resolution methods.
2. Extend `src/transform/native/types.rs` with helper methods that compute default values for each `FieldType` variant.
3. Wire the new module through `src/transform/native/mod.rs` and re-export it (with aliasing) from `src/transform/mod.rs`.
4. Write targeted unit tests in `tests/unit/native_field_definition_tests.rs` validating name rules, default mismatches, and generated defaults for nested types.
5. Document the architectural rule in `docs/project_logic.md` and mark the task as in progress within `docs/delivery/NTS-1/tasks.md`.
6. Run formatting, Rust workspace tests, clippy, and the repository-required frontend test suite.

## Verification
- Invalid field names (empty, whitespace, illegal characters) produce descriptive validation errors.
- Default values that do not match the declared type are rejected with typed errors.
- Optional fields without explicit defaults produce type-derived defaults (e.g., empty array/object, zero numbers).
- Nested object defaults recursively generate defaults for child field types.
- Rust and frontend test suites pass after introducing the new native field definition module.

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/project_logic.md`
- `src/transform/native/field_definition.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/types.rs`
- `src/transform/mod.rs`
- `tests/unit/mod.rs`
- `tests/unit/native_field_definition_tests.rs`

## Test Plan
- `cargo fmt` to ensure Rust style consistency.
- `cargo test --workspace` to execute all Rust unit and integration tests.
- `cargo clippy --workspace --all-targets --all-features` to enforce linting standards.
- `(cd src/datafold_node/static-react && npm install && npm test)` to satisfy required frontend checks.
