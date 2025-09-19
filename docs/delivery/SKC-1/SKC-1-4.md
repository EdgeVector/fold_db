# [SKC-1-4] UI helpers support universal key and consistent detection

[Back to task list](./tasks.md)

## Description
Update UI utilities to read optional `key` on Single/Range and required on HashRange; keep detection logic simple and aligned with backend.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:03:30 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements
- UI helpers expose `getHashKey`, `getRangeKey`, and `getKeyShape` uniformly.
- Prefer minimal additional logic; reuse Range logic per user preference.

## Implementation Plan
- Adjust `*SchemaHelpers.js` to read `schema.key` if present for Single/Range.
- Keep HashRange detection unchanged but compatible with universal `key`.

## Verification
- Unit tests in UI utils verifying detection and readout across all types.

## Files Modified
- `src/datafold_node/static-react/src/utils/*SchemaHelpers.js`
