# [SKC-1-3] Backend unify key extraction and result shaping

[Back to task list](./tasks.md)

## Description
Consolidate key extraction in backend and standardize query/mutation result shaping as hash->range->fields across schema types.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:03:00 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements
- Single entry point to compute hash and range values from `KeyConfig` (or legacy range_key).
- Standardize returned shape as `hash`, `range`, `fields` for all schema types.
- Maintain existing behavior where applicable; no breaking changes.

## Implementation Plan
- Introduce helper in backend to produce `(hash_opt, range_opt)` given schema + value.
- Update executor and schema operations to use the helper.

## Verification
- Integration tests for Single, Range, HashRange result shapes.
- Ensure preferred format is applied consistently.

## Files Modified
- `src/transform/executor.rs`
- `src/schema/schema_operations.rs`
