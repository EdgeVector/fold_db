# NTS-4-2 Implement ProcessingContext

[Back to task list](./tasks.md)

## Description
Add a strongly typed processing context to capture schema identity, inbound data, and the ordered transform chain. The context should make it easy for callers to build and inspect pipeline executions without manipulating raw maps directly.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 21:50:10 | Status Change | N/A | Proposed | Documented ProcessingContext task scope | AI_Agent |
| 2025-09-22 21:50:20 | Status Change | Proposed | In Progress | Began adding ProcessingContext API and helpers | AI_Agent |
| 2025-09-22 21:55:00 | Status Change | In Progress | Review | Context struct implemented with accessor and builder utilities | AI_Agent |

## Requirements
- Introduce a `ProcessingContext` struct holding schema name, input data, and transform specifications.
- Provide ergonomic helpers for constructing contexts, appending transforms, and splitting into parts.
- Ensure references to input data and transforms can be borrowed without cloning when possible.
- Keep the API generic over transform specification types to avoid premature coupling.

## Implementation Plan
1. Extend `src/transform/native/pipeline.rs` with the `ProcessingContext` definition and associated methods.
2. Confirm the context integrates with `NativeDataPipeline::process_data` while maintaining ownership semantics.
3. Export the new type via `src/transform/native/mod.rs` and `src/transform/mod.rs`.
4. Update the task index to reflect the work in progress and eventual review state.

## Verification
- Context creation accepts arbitrary schema names and input maps.
- `push_transform` appends additional stages without reallocating callers.
- `transform_specs()` exposes immutable access to the chain for inspection.
- `into_parts()` returns owned components for downstream processing.
- Compilation succeeds with the new generic bounds.

## Files Modified
- `docs/delivery/NTS-4/tasks.md`
- `src/transform/mod.rs`
- `src/transform/native/mod.rs`
- `src/transform/native/pipeline.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm test)`
