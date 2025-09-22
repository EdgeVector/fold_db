# NTS-3-1 Implement NativeTransformExecutor

[Back to task list](./tasks.md)

## Description
Implement the first version of the native transform execution engine that consumes strongly typed transform specifications and produces native `FieldValue` outputs. The executor must evaluate map-style transforms using native field definitions, enforce type guarantees, and surface precise errors so future tasks can extend the engine with additional transform kinds, expression handling, and function execution.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 21:40:30 | Status Change | N/A | Proposed | Task file created with initial analysis | AI_Agent |
| 2025-09-22 21:41:00 | Status Change | Proposed | InProgress | Began implementing native executor core and supporting specs | AI_Agent |
| 2025-09-22 22:30:00 | Status Change | InProgress | Review | Completed native executor implementation, documentation, and tests | AI_Agent |

## Requirements
- Provide a `NativeTransformExecutor` that accepts native transform specifications and input records of `FieldValue` values.
- Support map transforms with direct field mapping and constant value emission while validating outputs against `FieldDefinition` metadata.
- Return well-typed `FieldValue` objects without falling back to JSON conversions except at module boundaries.
- Emit descriptive errors for missing inputs, unsupported mappings, and type mismatches instead of silently coercing data.
- Introduce reusable transform specification structures to describe outputs, ensuring future tasks can extend them with new mapping variants.
- Add targeted unit tests that cover successful execution, default handling, and error scenarios.
- Document the new execution rule in `docs/project_logic.md` to keep architectural guidance in sync.

## Implementation Plan
1. Add a `transform_spec` module under `src/transform/native` that defines map transform structures (`MapTransform`, `MapField`, `FieldComputation`) and reusable type aliases for native records.
2. Create `src/transform/engine` with an `executor` module that exposes `NativeTransformExecutor` and `NativeTransformError` for map execution, validating outputs against `FieldDefinition` metadata and defaults.
3. Integrate the new engine by wiring the module through `src/transform/mod.rs` and re-exporting the executor for external callers.
4. Write unit tests covering direct input mapping, constant emission, optional/default handling, and error cases like missing inputs or type mismatches.
5. Update `docs/project_logic.md` with a `TRANSFORM-006` entry describing the native executor workflow and guarantees.
6. Run `cargo fmt`, `cargo test --workspace`, `cargo clippy --workspace --all-targets --all-features`, and the repository-mandated frontend test suite to ensure all tooling remains green.

## Verification
- Map transforms correctly propagate input values and constants while respecting field definitions and defaults.
- Type mismatches and missing required inputs surface as `NativeTransformError` variants with helpful context.
- Unit tests assert successful execution and failure paths.
- Repository formatting, linting, Rust tests, and frontend tests all pass after the implementation.

## Files Modified
- `docs/delivery/NTS-3/tasks.md`
- `docs/delivery/NTS-3/NTS-3-1.md`
- `docs/project_logic.md`
- `src/transform/mod.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/transform_spec.rs`
- `src/transform/engine/mod.rs`
- `src/transform/engine/executor.rs`
- `src/transform/hash_range_executor.rs`
- `src/fold_db_core/mutation_completion_handler.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm install && npm test)`
