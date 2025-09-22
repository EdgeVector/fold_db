# NTS-5-2 Implement conversion utilities

[Back to task list](./tasks.md)

## Description
Add type-safe conversion functions that provide schema-aware conversion between JSON and native types. These utilities will handle validation during conversion, proper error handling for invalid data, and ensure type safety throughout the conversion process.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 10:00:00 | Created | N/A | Proposed | Task file created with scope for conversion utilities | AI Agent |
| 2025-01-27 10:05:00 | Status Change | Proposed | Agreed | Task approved and ready for implementation | AI Agent |
| 2025-01-27 10:10:00 | Status Change | Agreed | InProgress | Started implementing conversion utilities | AI Agent |
| 2025-01-27 11:00:00 | Status Change | InProgress | Review | Implementation complete with comprehensive tests and documentation | AI Agent |
| 2025-01-27 11:05:00 | Status Change | Review | Done | Task completed and approved - all tests passing | AI Agent |

## Requirements
- Implement type-safe conversion functions for JSON to native FieldValue conversion
- Implement type-safe conversion functions for native FieldValue to JSON conversion
- Add schema-aware validation during conversion process
- Provide clear, typed error handling for conversion failures
- Ensure conversion utilities are reusable across the codebase
- Add support for optional field defaults during conversion
- Handle edge cases like null values, type mismatches, and missing required fields

## Implementation Plan
1. Create conversion utility functions in the existing `src/api/json_boundary.rs` module
2. Implement `json_to_native()` function with schema validation and type checking
3. Implement `native_to_json()` function with proper type enforcement
4. Add utility functions for handling optional fields and defaults
5. Create helper functions for common conversion patterns
6. Add comprehensive error types for conversion failures
7. Write focused unit tests covering conversion scenarios, validation, and error cases

## Verification
- Conversion utilities correctly convert JSON objects to native FieldValue maps
- Conversion utilities correctly convert native FieldValue maps to JSON objects
- Schema validation catches type mismatches and missing required fields
- Error handling provides clear, actionable error messages
- Optional field defaults are applied correctly during conversion
- Edge cases (null values, type coercion, etc.) are handled appropriately
- All unit tests pass for conversion scenarios

## Files Modified
- `docs/delivery/NTS-5/tasks.md`
- `docs/delivery/NTS-5/NTS-5-2.md`
- `docs/project_logic.md`
- `src/api/json_boundary.rs`
- `tests/unit/json_boundary_layer_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo test --workspace`
- `(cd fold_node/src/datafold_node/static-react && npm install)`
- `(cd fold_node/src/datafold_node/static-react && npm test)`
