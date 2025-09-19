# [SKC-1-2] Parser updates to accept key for all schema types

[Back to task list](./tasks.md)

## Description
Update JSON schema parsing to accept optional `key` for all schema types, with per-type validation and full backward compatibility for legacy Range.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:02:30 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 12:20:00 | Status Update | Proposed | InProgress | Started parser updates for universal key support | ai-agent |
| 2025-09-19 12:35:00 | Status Update | InProgress | Review | Parser updates complete - universal key parsing for all schema types with comprehensive tests | ai-agent |
| 2025-09-19 14:30:00 | Status Update | Review | Done | Task verified complete - parser supports universal key for all schema types | ai-agent |

## Requirements
- Parse `key` for all schema types.
- Do not require `key` for Single; require `range_field` for Range; require both for HashRange.
- Continue supporting `SchemaType::Range { range_key }` without `key`.

## Implementation Plan
- Extend `DeclarativeSchemaDefinition` and deserialization as needed.
- Update `validate_*` functions in `json_schema.rs` to enforce new rules.

## Verification
- Unit tests covering parsing with/without `key` for each schema type.
- Backward-compat tests for legacy Range-only `range_key`.

## Files Modified
- `src/schema/types/json_schema.rs`
