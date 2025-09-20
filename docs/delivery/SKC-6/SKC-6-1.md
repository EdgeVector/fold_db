# SKC-6-1 Introduce schema-driven key snapshot helper for field processing

## Description
Create a focused helper inside `field_processing.rs` that loads schema metadata and derives a normalized key snapshot using the universal key configuration helpers. Centralizing this logic removes ad-hoc key probing and gives later refactors a single entry point for retrieving `{hash, range, fields}` data for any schema type.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:00:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-23 09:05:00 | Scope Refined | Proposed | Proposed | Narrowed scope to helper introduction as part of task decomposition | ai-agent |

## Requirements
- Add a private helper (e.g. `resolve_universal_keys`) that receives the manager, schema name, and request payload, loads the schema via `db_ops.get_schema`, and returns a structured snapshot of hash/range/field data.
- Create a small data structure (struct or tuple) to hold `hash`, `range`, and the normalized `fields` map that other call sites can reuse without JSON probing.
- Use `extract_unified_keys()` and `shape_unified_result()` to resolve key names and values for Single, Range, and HashRange schemas, including dotted-path configurations.
- Translate failures from the universal helpers into descriptive errors that preserve context (schema, field, underlying issue) without falling back to silent defaults.
- Ensure the helper leaves existing call sites untouched for now but is fully covered by unit tests to support subsequent refactors.

## Implementation Plan
1. Define the `ResolvedAtomKeys` struct (or similar) adjacent to existing field-processing types to encapsulate key data returned by the helper.
2. Implement `resolve_universal_keys(manager, request)` to fetch the schema, call the universal helper functions, and build the normalized payload snapshot.
3. Map errors from schema lookup or universal helper calls into the existing `FieldValueSet` error types, enriching messages with schema and field context.
4. Add a thin instrumentation wrapper (debug log or trace) that confirms when universal key resolution succeeds or fails, ensuring no `println!` diagnostics remain.

## Test Plan
- Add unit tests under `tests/unit/field_processing/` that exercise the helper for Single, Range (legacy + universal), HashRange, and dotted-path key schemas using fixtures from `tests/test_utils.rs`.
- Add negative tests covering missing key configuration, missing data, and schema lookup failures to verify descriptive errors.
- Run `cargo test --workspace` and `cargo clippy --all-targets --all-features` to confirm the helper and tests compile cleanly.

## Verification
- The new helper returns the expected `hash`, `range`, and `fields` values for every schema type without relying on ad-hoc JSON probing.
- Error scenarios surface actionable messages and no longer fall back to silent defaults.
- Existing behavior remains unchanged until downstream tasks adopt the helper.

## Files Modified
- `src/fold_db_core/managers/atom/field_processing.rs`
- `tests/unit/field_processing/universal_key_helper_tests.rs`
- `tests/test_utils.rs`

[Back to task list](../tasks.md)
