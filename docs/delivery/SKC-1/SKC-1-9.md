# [SKC-1-9] Retire redundant UI detection code paths

[Back to task list](./tasks.md)

## Description
Delete specialized UI detection logic in favor of universal key-based helpers; keep logic minimal and DRY per user preference.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:07:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 16:45:00 | Status Update | Proposed | InProgress | Start retiring redundant UI detection code paths | ai-agent |
| 2025-09-19 17:00:00 | Status Update | InProgress | Review | Consolidated UI helpers, updated imports, deleted redundant files, all tests passing | ai-agent |

## Requirements
- Use a single set of helpers for `getHashKey`, `getRangeKey`, and detection.
- Remove duplicate/legacy code in `hashRangeSchemaUtils` if superseded.

## Implementation Plan
- Update helpers; replace imports in UI components.
- Delete dead utilities after replacement.

## Verification
- UI builds; unit tests for helpers pass.

## Files Modified
- `src/datafold_node/static-react/src/utils/*`
