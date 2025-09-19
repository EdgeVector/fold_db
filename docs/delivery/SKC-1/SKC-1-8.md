# [SKC-1-8] Consolidate JSON readers for key config

[Back to task list](./tasks.md)

## Description
Remove duplicate JSON readers for `key` config and centralize into one module with robust error messages and tests.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 12:06:30 | Created | N/A | Proposed | Task file created | ai-agent |

## Requirements
- Single function to read `key` from schema JSON files.
- Clear errors when `key` is malformed; tolerant when absent for Single.

## Implementation Plan
- Replace `get_hashrange_key_config_from_json` and similar with unified reader.
- Update call sites to new API.

## Verification
- Unit tests for present/missing/malformed key across schema types.

## Files Modified
- `src/schema/schema_operations.rs`
- `src/fold_db_core/transform_manager/schema_data_fetcher.rs`
