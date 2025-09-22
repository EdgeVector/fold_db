# NTS-6-2 Implement database format conversion

[Back to task list](./tasks.md)

## Description
Deliver conversion utilities that bridge native `FieldValue` payloads with the sled storage format. The utilities must enforce typed schemas, apply defaults, serialise composite keys safely, and recover native values when loading records.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-25 09:45:00 | Status Change | N/A | In Progress | Started implementing conversion helpers for native persistence. | AI_Agent |
| 2025-09-25 11:45:00 | Status Change | In Progress | Done | Finalised conversion logic and integrated validation plus key serialisation. | AI_Agent |

## Requirements
- Provide helpers to convert native maps into JSON values for storage without exposing JSON to callers.
- Support round-trip conversion from persisted JSON to native `FieldValue` instances.
- Apply schema-driven defaults for optional fields and validate required fields during load.
- Encode composite keys deterministically and losslessly for sled storage.
- Surface descriptive errors when conversion or validation fails.

## Implementation Plan
1. Add `convert_to_db_format` and `convert_from_db_format` helpers inside `NativePersistence` to manage JSON boundaries.
2. Implement `normalize_and_validate` to apply defaults and reject payloads that violate schema definitions.
3. Add key serialisation helpers (base64-encoded segments) to guarantee reversible storage keys.
4. Integrate conversion helpers with `store_data` and `load_data` paths to enforce validation before and after persistence.
5. Extend integration tests to verify round-trip behaviour, error cases, and default application.

## Verification
- Native records persist and reload with optional defaults applied automatically.
- Type mismatches and missing required fields return `PersistenceError` variants instead of silent failures.
- Composite key records round-trip with identical hash/range values.
- Unit tests confirm key serialisation is reversible.
- Workspace and frontend test suites pass after the conversion utilities are integrated.

## Files Modified
- `docs/delivery/NTS-6/tasks.md`
- `docs/delivery/NTS-6/NTS-6-2.md`
- `src/persistence/native_persistence.rs`
- `tests/native_persistence_tests.rs`

## Test Plan
- `rustfmt src/persistence/mod.rs src/persistence/native_persistence.rs tests/native_persistence_tests.rs`
- `cargo test --workspace`
- `cargo clippy --workspace`
- `(cd src/datafold_node/static-react && npm ci && npm test)`
