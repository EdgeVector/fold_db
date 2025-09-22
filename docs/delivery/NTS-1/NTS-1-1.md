# NTS-1-1 Implement FieldValue and FieldType enums

[Back to task list](./tasks.md)

## Description
Implement the foundational native data representations that will replace the current JSON-centric values inside the transform system. This task introduces strongly typed enums for values and their declared types so later tasks can build native field definitions, transform specifications, and execution paths without relying on `serde_json::Value` everywhere.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 19:14:07 | Status Change | N/A | Proposed | Task file created and initial analysis started | AI_Agent |
| 2025-09-22 19:14:30 | Status Change | Proposed | In Progress | Began implementing native value and type enums | AI_Agent |
| 2025-09-23 09:30:00 | Status Change | In Progress | Review | Implementation complete with comprehensive tests | AI_Agent |
| 2025-09-23 09:35:00 | Status Change | Review | Done | Task completed and approved - all tests passing | AI_Agent |

## Requirements
- Provide `FieldValue` enum that captures supported native value kinds (string, numeric, boolean, array, object, null).
- Provide `FieldType` enum that captures expected type declarations for schema fields, including nested array/object metadata.
- Support round-trip conversion between the native enums and `serde_json::Value` for boundary interactions.
- Offer helper methods for basic introspection (`field_type`) and validation (`matches`).
- Ensure enums derive `Serialize`/`Deserialize` for persistence and future API compatibility.
- Include thorough unit tests that cover conversions, matching semantics, and edge cases (empty arrays, nested objects, null handling).

## Implementation Plan
1. Create `src/transform/native` module with `types.rs` defining `FieldValue` and `FieldType` enums plus helper functions.
2. Add module wiring to `src/transform/mod.rs` to expose the new native module and re-export the enums for callers.
3. Implement conversion helpers (`to_json_value`, `from_json_value`) and type inspection utilities on `FieldValue` alongside `FieldType::matches`.
4. Introduce focused unit tests under `tests/unit` validating conversions, inferred element typing, and matching behavior for common and nested structures.
5. Update `docs/project_logic.md` with the new native type rule to keep architectural documentation aligned.
6. Run formatting, Rust workspace tests, clippy, and UI vitest suite to confirm the change integrates cleanly.

## Verification
- `FieldValue` correctly infers nested array/object element types and preserves data through JSON round-trips.
- `FieldType::matches` accepts valid combinations (including null values for optional fields) and rejects mismatches.
- New unit tests cover representative cases and guard against regressions.
- Cargo workspace builds cleanly with `cargo fmt`, `cargo test --workspace`, and `cargo clippy`.
- Frontend vitest suite (`npm test`) continues to pass, confirming the broader workspace remains healthy.

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/project_logic.md`
- `src/transform/mod.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/types.rs`
- `tests/unit/native_types_tests.rs`

## Test Plan
- `cargo fmt` to maintain style before running tests.
- `cargo test --workspace` to execute Rust unit and integration suites with the new native types.
- `cargo clippy --workspace --all-targets --all-features` to enforce lint hygiene for the new module.
- `(cd src/datafold_node/static-react && npm install && npm test)` to satisfy repository policy on frontend tests.
