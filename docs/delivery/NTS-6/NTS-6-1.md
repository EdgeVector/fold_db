# NTS-6-1 Implement NativePersistence

[Back to task list](./tasks.md)

## Description
Introduce a native persistence layer that accepts strongly typed `FieldValue` payloads, validates them against typed schema metadata, and stores them using the existing sled-backed `DbOperations`. This removes the need for callers to manipulate raw `serde_json::Value` instances and establishes the foundation for native data flows across the node.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-25 09:30:00 | Status Change | N/A | In Progress | Began implementing native persistence module and schema provider abstraction. | AI_Agent |
| 2025-09-25 11:45:00 | Status Change | In Progress | Done | Completed native persistence implementation and validation utilities. | AI_Agent |

## Requirements
- Provide a `NativePersistence` helper that stores and loads native records without exposing JSON to callers.
- Support single-key and composite (hash/range) schemas with deterministic key serialization.
- Validate input payloads against typed schema metadata before persistence and reject mismatches.
- Integrate with existing `DbOperations` for storage while minimising JSON usage to conversion boundaries.
- Ensure interfaces remain thread-safe for use inside async managers.

## Implementation Plan
1. Create `src/persistence` module to host native persistence helpers and re-export them through `lib.rs`.
2. Implement `NativeSchemaProvider`, `SchemaDescription`, and `KeyConfig` to describe typed schema metadata required for persistence.
3. Build `NativePersistence` with validation, key extraction, and storage helpers that utilise `DbOperations`.
4. Provide `NativeRecordKey` utilities to serialise composite keys safely using base64 encoding.
5. Add module-level unit tests to ensure key round-tripping and error paths operate as expected.
6. Write integration tests that persist and load records through the new helper using in-memory schema definitions.
7. Update documentation (`project_logic.md`, task files) to capture the new persistence guarantees.

## Verification
- Native persistence stores records and loads them back with defaults applied for optional fields.
- Attempts to persist data with missing required fields or type mismatches surface explicit `PersistenceError` variants.
- Composite key schemas round-trip correctly with deterministic hash/range components.
- Workspace builds succeed with `cargo test --workspace` and `cargo clippy --workspace`.
- Frontend vitest suite runs successfully to maintain repository policy compliance.

## Files Modified
- `docs/delivery/NTS-6/tasks.md`
- `docs/delivery/NTS-6/NTS-6-1.md`
- `docs/project_logic.md`
- `src/lib.rs`
- `src/persistence/mod.rs`
- `src/persistence/native_persistence.rs`
- `tests/native_persistence_tests.rs`

## Test Plan
- `rustfmt src/persistence/mod.rs src/persistence/native_persistence.rs tests/native_persistence_tests.rs`
- `cargo test --workspace`
- `cargo clippy --workspace`
- `(cd src/datafold_node/static-react && npm ci && npm test)`
