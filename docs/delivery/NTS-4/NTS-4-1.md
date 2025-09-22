# NTS-4-1 Implement NativeDataPipeline

[Back to task list](./tasks.md)

## Description
Define the native data pipeline structure that orchestrates transform execution without falling back to JSON conversions. The pipeline must expose a clean API for running individual transforms or full transform chains and surface typed errors when execution fails.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 21:42:00 | Status Change | N/A | Proposed | Captured task scope and created documentation | AI_Agent |
| 2025-09-22 21:42:10 | Status Change | Proposed | In Progress | Began implementing native pipeline structure and error handling | AI_Agent |
| 2025-09-22 21:50:00 | Status Change | In Progress | Review | Pipeline struct and error type implemented, ready for review | AI_Agent |

## Requirements
- Introduce a `NativeDataPipeline` struct that accepts a transform engine and schema registry handles.
- Define a reusable `NativeTransformExecutor` trait that the pipeline can invoke.
- Surface pipeline-specific errors when transform stages fail or yield incompatible output shapes.
- Re-export the pipeline types through existing module entry points for crate consumers.
- Record architectural guidance for the native pipeline in `docs/project_logic.md`.

## Implementation Plan
1. Create `src/transform/native/pipeline.rs` housing the pipeline struct, error enum, executor trait, and execution helpers.
2. Update `src/transform/native/mod.rs` to wire the new module and re-export key types.
3. Extend `src/transform/mod.rs` to expose the pipeline API to downstream callers.
4. Refresh `docs/project_logic.md` with a new logic entry covering native pipeline orchestration rules.
5. Update the NTS-4 task index to reflect the in-progress status and prepare for subsequent tasks.

## Verification
- Pipeline construction succeeds with shared `Arc` handles for engine and registry.
- Transform execution errors bubble up as `PipelineError::Transform` values that include the failing stage index.
- Non-object transform outputs cause `PipelineError::NonObjectOutput` failures.
- Re-exported pipeline types compile across the crate without missing imports.
- Project logic entry accurately documents the new behavior.

## Files Modified
- `docs/delivery/NTS-4/tasks.md`
- `docs/project_logic.md`
- `src/transform/mod.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/pipeline.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm test)`
