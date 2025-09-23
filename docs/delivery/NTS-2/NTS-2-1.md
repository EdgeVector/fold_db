# NTS-2-1 Implement NativeSchema struct

[Back to task list](./tasks.md)

## Description
Create the foundational native schema representation that replaces JSON-centric
structs with strongly typed field definitions. The struct must capture schema
metadata, enforce key configuration invariants, and provide validation utilities
for downstream registry and boundary components planned in later tasks.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-24 10:05:00 | Status Change | N/A | Proposed | Task file created with initial scope review | AI_Agent |
| 2025-09-24 10:10:00 | Status Change | Proposed | In Progress | Began implementing native schema module and error types | AI_Agent |
| 2025-09-24 12:45:00 | Status Change | In Progress | Review | Implementation complete with tests and documentation updates | AI_Agent |

## Requirements
- Introduce a native schema module under `src/schema` that exposes `NativeSchema`
  and `KeyConfig` types with typed field maps.
- Enforce key configuration invariants (field existence, required flags, and
  prohibiting null-only key types) via structured errors.
- Provide payload validation helpers that flag unknown fields, missing required
  data, and type mismatches while supporting default population for optional
  fields.
- Supply a builder API for assembling schemas that performs eager validation and
  prevents duplicate registrations.
- Cover the new behaviour with comprehensive unit tests exercising success and
  failure scenarios.
- Update architectural logic documentation to record the new native schema rule
  and ensure task tracking reflects progress.

## Implementation Plan
1. Create `src/schema/native` module with `mod.rs` re-exporting the schema
   primitives.
2. Implement `NativeSchema`, `KeyConfig`, builder, and error types inside
   `schema.rs`, including payload validation and normalisation helpers.
3. Wire the new module through `src/schema/mod.rs` so callers can import native
   schema types from the crate root.
4. Add focused unit tests in `tests/unit/native_schema_tests.rs` covering builder
   failures, payload validation, and default normalisation behaviour.
5. Register the new test module in `tests/unit/mod.rs` and keep documentation in
   `docs/project_logic.md` and `docs/delivery/NTS-2/tasks.md` synchronized.
6. Run formatting, Rust workspace tests, clippy, and the repository-required
   frontend vitest suite.

## Verification
- Builder rejects duplicate, invalid, or missing key field definitions with the
  expected error variants.
- `validate_payload` accepts valid data and surfaces descriptive errors for
  unknown fields, missing required fields, and type mismatches.
- `normalise_payload` and `project_payload` populate optional fields with typed
  defaults without mutating the original payload map.
- Unit tests in `tests/unit/native_schema_tests.rs` cover success and failure
  paths for schema construction and payload handling.
- All mandated formatting and test commands pass after the changes.

## Files Modified
- `docs/delivery/NTS-2/tasks.md`
- `docs/delivery/NTS-2/NTS-2-1.md`
- `docs/project_logic.md`
- `src/schema/mod.rs`
- `src/schema/native/mod.rs`
- `src/schema/native/schema.rs`
- `tests/unit/mod.rs`
- `tests/unit/native_schema_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm ci)`
- `(cd src/datafold_node/static-react && npm test)`
