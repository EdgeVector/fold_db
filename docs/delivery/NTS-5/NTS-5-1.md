# NTS-5-1 Implement JsonBoundaryLayer

[Back to task list](./tasks.md)

## Description
Establish the initial JSON boundary layer that keeps existing HTTP APIs compatible while the internal system adopts native transform types. The boundary must register schema definitions, validate payloads against native field metadata, and convert between `serde_json::Value` payloads and native `FieldValue` maps with deterministic defaults.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:00:00 | Created | N/A | Proposed | Task file created with initial scope for JsonBoundaryLayer | AI Agent |
| 2025-09-23 09:05:00 | Status Change | Proposed | Agreed | Scope confirmed and work authorised for boundary implementation | AI Agent |
| 2025-09-23 09:10:00 | Status Change | Agreed | InProgress | Began implementing schema registration and conversion logic | AI Agent |
| 2025-09-23 15:30:00 | Status Change | InProgress | Review | Implementation complete with tests and documentation awaiting review | AI Agent |

## Requirements
- Provide a reusable `JsonBoundaryLayer` that registers schemas and enforces typed conversions.
- Ensure schema registration validates `FieldDefinition` invariants and catches configuration mistakes.
- Convert JSON payloads into native `FieldValue` maps while applying optional defaults and rejecting missing required fields.
- Convert native maps back into JSON objects, preserving defaults and blocking unknown fields unless explicitly allowed.
- Surface descriptive, typed errors for schema lookups, validation issues, and conversion mismatches.

## Implementation Plan
1. Add a new `api` module exposing the boundary layer public API surface.
2. Implement `JsonBoundaryLayer`, `JsonBoundarySchema`, and `JsonBoundaryError` with schema registration plus JSON/native conversion logic.
3. Write focused unit tests covering happy paths, default handling, unknown field rejection, error propagation, and opt-in passthrough behaviour.
4. Document the new boundary logic in `docs/project_logic.md` and update task tracking metadata.

## Verification
- Schema registration rejects definitions with mismatched defaults and mismatched field names.
- JSON payloads convert to native maps with defaults applied and unknown fields rejected by default.
- Native maps convert back to JSON with type enforcement and defaults for omitted optional fields.
- Allowing additional fields enables passthrough in both conversion directions.
- Unit tests covering the scenarios above all pass.

## Files Modified
- `docs/delivery/NTS-5/tasks.md`
- `docs/delivery/NTS-5/NTS-5-1.md`
- `docs/project_logic.md`
- `src/api/json_boundary.rs`
- `src/api/mod.rs`
- `src/lib.rs`
- `tests/unit/json_boundary_layer_tests.rs`
- `tests/unit/mod.rs`

## Test Plan
- `cargo fmt`
- `cargo clippy --workspace --all-targets --all-features`
- `cargo test --workspace`
- `(cd fold_node/src/datafold_node/static-react && npm install)`
- `(cd fold_node/src/datafold_node/static-react && npm test)`
