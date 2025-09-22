# NTS-1-3 Implement TransformSpec with native types

[Back to task list](./tasks.md)

## Description
Establish a native `TransformSpec` module that replaces loosely typed JSON transform descriptions. The specification must support map, filter, reduce, and chain behaviours while enforcing consistent validation and error reporting so later execution work can rely on trusted inputs.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 19:18:00 | Status Change | N/A | Proposed | Documented task scope and validation goals for native transform specifications | AI_Agent |
| 2025-09-22 19:18:30 | Status Change | Proposed | In Progress | Began implementing native transform spec module and shared identifier validation | AI_Agent |
| 2025-09-22 19:19:30 | Status Change | In Progress | Review | Completed implementation, documentation, and tests; awaiting review | AI_Agent |

## Requirements
- Define `TransformSpec` and supporting enums/structs (`TransformType`, `MapTransform`, `FilterTransform`, `ReduceTransform`, `FieldMapping`, `FilterCondition`, `Reducer`) using native types and Serde serialization.
- Provide comprehensive validation with a `TransformSpecError` enum to enforce identifier rules, mapping completeness, input references, and reducer/group-by constraints.
- Re-export native transform spec types through `transform::native` and the top-level `transform` module for downstream consumers.
- Extend identifier validation utilities so transform specs reuse the same naming guarantees as field definitions.
- Add unit tests covering successful validation and key failure cases (unknown references, constant mismatches, duplicate group-by fields, nested chain propagation).
- Update architectural documentation (`docs/project_logic.md`) to record the new native transform specification rule.

## Implementation Plan
1. Factor identifier validation helpers from `FieldDefinition` so they can be shared by transform specs without duplicating logic.
2. Create `src/transform/native/transform_spec.rs` with the native data structures, serde annotations, and validation routines plus the `TransformSpecError` type.
3. Wire the new module through `src/transform/native/mod.rs` and `src/transform/mod.rs`, ensuring all necessary types are publicly re-exported with `Native*` aliases.
4. Write focused unit tests (`tests/unit/native_transform_spec_tests.rs`) exercising validation success paths and representative failure scenarios.
5. Document the architecture update in `docs/project_logic.md` and refresh task metadata/status files to reflect progress.

## Verification
- Transform specifications with valid inputs, outputs, and mappings pass validation while invalid configurations produce descriptive `TransformSpecError` variants.
- Map transforms reject unknown input references and constant type mismatches based on the declared output field types.
- Filter and reduce transforms validate referenced fields and enforce non-empty logical groups.
- Chain transforms propagate nested validation failures with context so downstream consumers can surface precise errors.
- Rust and frontend test suites pass after introducing the new module.

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/delivery/NTS-1/NTS-1-3.md`
- `docs/project_logic.md`
- `src/transform/native/field_definition.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/transform_spec.rs`
- `src/transform/mod.rs`
- `tests/unit/mod.rs`
- `tests/unit/native_transform_spec_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm install && npm test)`
