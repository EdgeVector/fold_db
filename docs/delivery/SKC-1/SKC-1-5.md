# [SKC-1-5] Docs and migration guide for universal key

[Back to task list](./tasks.md)

## Description
Document the universal `key` format and provide migration guidance with examples for Single, Range, and HashRange.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:04:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements
- Update docs with universal `key` examples for each schema type.
- Migration notes for legacy Range and schemas without `key`.

## Implementation Plan
- Update `docs/schema-management.md` and related references.
- Add a short migration guide and cross-link from PRD.

## Verification
- Documentation lint passes; links valid.

## Files Modified
- `docs/schema-management.md`
- `docs/delivery/SKC-1/prd.md`
