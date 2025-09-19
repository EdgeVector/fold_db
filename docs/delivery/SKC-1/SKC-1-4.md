# [SKC-1-4] UI helpers support universal key and consistent detection

[Back to task list](./tasks.md)

## Description
Update UI utilities to read optional `key` on Single/Range and required on HashRange; keep detection logic simple and aligned with backend.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:03:30 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 13:15:00 | Status Update | Proposed | InProgress | Start UI helpers update for universal key support | ai-agent |
| 2025-09-19 13:55:00 | Status Update | InProgress | InProgress | Implemented getHashKey, extended getRangeKey for Single/Range/HashRange, added getKeyShape, and normalized dotted paths. | ai-agent |

## Requirements
- UI helpers expose `getHashKey`, `getRangeKey`, and `getKeyShape` uniformly.
- Prefer minimal additional logic; reuse Range logic per user preference.

## Implementation Plan
- Adjust `*SchemaHelpers.js` to read `schema.key` if present for Single/Range.
- Keep HashRange detection unchanged but compatible with universal `key`.

## Verification
- Unit tests in UI utils verifying detection and readout across all types.
- Lint clean for updated helpers.

## Files Modified
- `src/datafold_node/static-react/src/utils/*SchemaHelpers.js`
  - `rangeSchemaHelpers.js`: new `getHashKey`, enhanced `getRangeKey`, `getKeyShape`, internal `lastSegment`, fixed param name in `formatHashRangeQuery`.
