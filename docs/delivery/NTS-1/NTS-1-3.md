# NTS-1-3 Implement TransformSpec with native types

[Back to task list](./tasks.md)

## Description
Build native transform specifications that pair the new field types with declarative mapping metadata. The specification layer replaces JSON blobs with strongly typed structures that describe inputs, outputs, and transform operations (map, filter, reduce, chain). This enables later execution tasks to consume rich native metadata without repeated JSON parsing.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-23 09:30:00 | Status Change | N/A | Proposed | Task file created with initial analysis for native transform specifications | AI_Agent |
| 2025-09-23 09:32:00 | Status Change | Proposed | In Progress | Began implementing native transform specification module and supporting validation | AI_Agent |
| 2025-09-23 11:45:00 | Status Change | In Progress | Review | Submitted native transform specification module with unit tests for review | AI_Agent |
| 2025-09-23 12:00:00 | Status Change | Review | Done | Task completed and approved - all tests passing | AI_Agent |

## Requirements
- Define native `TransformSpec` data structures that capture transform name, inputs, output, and transform kind (map, filter, reduce, chain).
- Provide serde serialization for the new structures so they can be persisted or exchanged when needed.
- Implement validation logic that enforces unique inputs, validates field definitions, and checks field references within mappings, conditions, reducers, and chains.
- Propagate typed errors describing validation failures (unknown field references, empty mappings, empty condition groups, etc.).
- Re-export the new types through the `transform` module for downstream consumers.
- Add comprehensive unit tests covering success and failure scenarios for map, filter, reduce, and chain specifications.
- Document the architectural rule in `docs/project_logic.md` to reflect the new native transform specification contract.

## Implementation Plan
1. Create `src/transform/native/transform_spec.rs` implementing the data structures, serde support, and validation helpers.
2. Update `src/transform/native/mod.rs` and `src/transform/mod.rs` to expose the new types to the rest of the codebase.
3. Write unit tests in `tests/unit/native_transform_spec_tests.rs` verifying validation across all transform kinds.
4. Update task tracking metadata and add a new logic entry to `docs/project_logic.md` describing the native transform specification requirement.
5. Run formatting, linting, Rust tests, and the required frontend vitest suite.

## Verification
- `TransformSpec::validate` accepts well-formed map, filter, reduce, and chain specifications.
- Validation rejects unknown field references, empty mapping collections, empty condition groups, and reducer variants missing source fields.
- Chain validation surfaces nested errors with contextual indices.
- New unit tests cover the success and failure cases noted above.
- Repository linting and automated test suites all pass.

## Files Modified
- `docs/delivery/NTS-1/tasks.md`
- `docs/project_logic.md`
- `src/transform/native/mod.rs`
- `src/transform/native/transform_spec.rs`
- `src/transform/mod.rs`
- `tests/unit/mod.rs`
- `tests/unit/native_transform_spec_tests.rs`

## Test Plan
- `cargo fmt`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features`
- `(cd src/datafold_node/static-react && npm install)`
- `(cd src/datafold_node/static-react && npm test)`
