# [SKC-1-1] Define universal KeyConfig and validation rules

[Back to task list](./tasks.md)

## Description
Introduce a universal `KeyConfig` structure usable by Single, Range, and HashRange. Specify per-type validation rules ensuring backward compatibility.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:02:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 12:05:00 | Status Update | Proposed | InProgress | Started implementation for SKC-1-1 | ai-agent |
| 2025-09-19 12:15:00 | Status Update | InProgress | Review | Implementation complete - universal KeyConfig with validation rules | ai-agent |

## Requirements
- Define `KeyConfig { hash_field?: string, range_field?: string }` for all schema types.
- Validation rules:
  - Single: `key` optional; if present, may include either/both; no strict requirements.
  - Range: `range_field` required; `hash_field` optional.
  - HashRange: both required (unchanged).
- Preserve legacy `SchemaType::Range { range_key }` behavior.

## Implementation Plan
- Update Rust structs and serde attributes where needed.
- Add validation functions scoped by `SchemaType`.
- Maintain compatibility with existing JSON schema files.

## Verification
- Unit tests covering validation rules for Single, Range, HashRange with/without `key`.
- Ensure existing tests still pass.

## Files Modified
- `src/schema/types/json_schema.rs`
- `src/schema/types/schema.rs`
