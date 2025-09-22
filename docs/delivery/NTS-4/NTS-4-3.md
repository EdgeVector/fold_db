# NTS-4-3 Add transform chain execution

[Back to task list](./tasks.md)

## Description
Implement the core execution loop that feeds native data through each transform specification in order. The pipeline must halt on failures, enforce object outputs for chaining, and deliver the final native map for downstream persistence.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-22 21:55:10 | Status Change | N/A | Proposed | Defined transform chain execution task | AI_Agent |
| 2025-09-22 21:55:20 | Status Change | Proposed | In Progress | Started implementing sequential execution over transform specs | AI_Agent |
| 2025-09-22 22:00:00 | Status Change | In Progress | Review | Transform chain execution complete with error propagation | AI_Agent |

## Requirements
- Iterate over each transform specification in the provided context sequentially.
- Invoke the executor for every stage and propagate failures immediately.
- Require each stage to return an object map for continued chaining; otherwise emit a descriptive error.
- Include the index of any failing transform stage in surfaced errors to simplify debugging.
- Return the final native map when all transforms succeed.
- Keep the implementation generic over transform specification types.

## Implementation Plan
1. Update `NativeDataPipeline::process_data` to consume the context and evaluate transform specs in order.
2. Map executor failures into `PipelineError::Transform` to preserve root causes.
3. Derive the offending field type when a stage yields a non-object value and wrap it in `PipelineError::NonObjectOutput`.
4. Ensure the function returns the last successful object map without unnecessary cloning.

## Verification
- Successful chains return the final transform's object payload.
- A non-object output triggers a `PipelineError::NonObjectOutput` with the correct index and type.
- Executor errors surface as `PipelineError::Transform` variants that report the failing stage index.
- Empty transform lists simply return the original input map.

## Files Modified
- `docs/delivery/NTS-4/tasks.md`
- `src/transform/native/pipeline.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm test)`
