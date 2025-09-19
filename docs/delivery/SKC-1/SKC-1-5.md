# [SKC-1-5] Docs and migration guide for universal key

[Back to task list](./tasks.md)

## Description
Document the universal `key` format and provide migration guidance with examples for Single, Range, and HashRange.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:04:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 14:35:00 | Status Update | Proposed | InProgress | Start documentation and migration guide for universal key | ai-agent |
| 2025-09-19 15:00:00 | Status Update | InProgress | Review | Documentation complete - updated schema-management.md and created comprehensive migration guide | ai-agent |
| 2025-09-19 17:25:00 | Status Update | Review | Done | Task verified complete - comprehensive documentation and migration guide created | ai-agent |

## Requirements
- Update docs with universal `key` examples for each schema type.
- Migration notes for legacy Range and schemas without `key`.

## Implementation Plan
- Update `docs/reference/schema-management.md` and related references.
- Add a short migration guide and cross-link from PRD.

## Verification
- Documentation lint passes; links valid.

## Files Modified
- `docs/reference/schema-management.md`
- `docs/delivery/SKC-1/prd.md`
