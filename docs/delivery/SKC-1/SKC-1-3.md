# [SKC-1-3] Backend unify key extraction and result shaping

[Back to task list](./tasks.md)

## Description
Consolidate key extraction in backend and standardize query/mutation result shaping as hash->range->fields across schema types.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:03:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 12:40:00 | Status Update | Proposed | InProgress | Started backend key extraction unification | ai-agent |
| 2025-09-19 13:10:00 | Status Update | InProgress | Review | Unified key extraction and result shaping implemented, tests and clippy pass | ai-agent |
| 2025-09-19 13:45:00 | Status Update | Review | Done | Backend helpers implemented and verified; marking complete to proceed with UI task | ai-agent |

## Requirements
- Single entry point to compute hash and range values from `KeyConfig` (or legacy range_key).
- Standardize returned shape as `hash`, `range`, `fields` for all schema types.
- Maintain existing behavior where applicable; no breaking changes.

## Implementation Plan
- Introduced `extract_unified_keys(schema, data) -> (Option<String>, Option<String>)` in `src/schema/schema_operations.rs`.
- Added `shape_unified_result(schema, data, hash, range) -> serde_json::Value` to return `{ hash, range, fields }` consistently.
- Updated tests to include `key: None` on `Schema` initializers and to use `KeyConfig` with String fields (empty means absent).

## Verification
- Ran `cargo test --workspace`: all suites passed.
- Ran `cargo clippy`: clean.
- Ran frontend tests in `static-react`: all passed.

## Files Modified
- `src/schema/schema_operations.rs` (added `extract_unified_keys`, `shape_unified_result`).
- `tests/unit/unified_key_extraction_tests.rs` (aligned to String fields; added shaping checks).
- `tests/unit/hashrange_schema_tests.rs`, `tests/database_regression_prevention.rs` (added `key: None`).
