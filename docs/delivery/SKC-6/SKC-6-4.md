# SKC-6-4 Retire legacy key heuristics and tighten error reporting

## Description
After all field processing paths consume the universal key snapshot, remove the legacy `extract_range_key_from_value` and `extract_hash_key_from_value` helpers and consolidate error handling. The goal is to eliminate dead code, reduce branching, and ensure all failures leverage the new descriptive error surface.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-21 12:15:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-23 09:20:00 | Scope Refined | Proposed | Proposed | Converted task into cleanup and error-hardening step | ai-agent |

## Requirements
- Delete `extract_range_key_from_value`, `extract_hash_key_from_value`, and any other unused heuristic utilities from `field_processing.rs` and related modules.
- Remove diagnostic `println!` statements and replace remaining ad-hoc logging with structured `tracing`/`log` calls that summarize schema, field, and error context.
- Ensure all error paths in field processing derive from the universal helper's error types, providing actionable messages without silent fallbacks.
- Update documentation comments within `field_processing.rs` to reflect the simplified architecture and point to the universal key helper.
- Confirm no remaining call sites depend on the deleted helpers before removing them.

## Implementation Plan
1. Search for references to the legacy heuristic functions and confirm the earlier refactor tasks eliminated their usage.
2. Delete the obsolete helper definitions along with redundant JSON manipulation code, updating imports accordingly.
3. Normalize logging and error propagation in the affected functions to use the shared error types introduced alongside the universal helper.
4. Refresh inline documentation and ensure the module-level comment explains the new flow.

## Test Plan
- Run the full suite `cargo test --workspace` to ensure no regressions after removing the helpers.
- Run `cargo clippy --all-targets --all-features` to catch unused-code or lint issues introduced by deletions.
- Spot-check the Single, Range, and HashRange integration tests to verify they continue to pass without the legacy code.

## Verification
- No references to the legacy heuristic helpers remain in the codebase.
- Field processing errors surface descriptive messages for missing key configuration or malformed payloads.
- Logging is consistent, structured, and free of debugging `println!` calls.

## Files Modified
- `src/fold_db_core/managers/atom/field_processing.rs`
- `tests/unit/field_processing/*`
- `tests/integration/hashrange_end_to_end_workflow_test.rs`

[Back to task list](../tasks.md)
