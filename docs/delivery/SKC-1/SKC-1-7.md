# [SKC-1-7] Remove legacy Range { range_key } branching in backend

[Back to task list](./tasks.md)

## Description
Replace ad-hoc backend branches that special-case `Range { range_key }` with calls to a unified key helper. Preserve parsing/backward compatibility but simplify execution code paths.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:06:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 16:00:00 | Status Update | Proposed | InProgress | Start removing legacy Range { range_key } branching in backend | ai-agent |

## Requirements
- Identify and replace code that checks schema variants directly for range handling.
- Use the unified key extraction helper everywhere keys are needed.
- No behavior regressions.

## Implementation Plan
- Grep for `SchemaType::Range` usages in execution, ops, and services.
- Introduce/consume a single helper that returns `(hash_opt, range_opt)`.

## Verification
- All integration tests pass; add focused tests where needed.

## Files Modified
- `src/transform/executor.rs`
- `src/fold_db_core/transform_manager/*`
- `src/schema/schema_operations.rs`
