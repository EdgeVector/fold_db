# [SKC-2-1] Update HashRange query processor to use universal key configuration

[Back to task list](./tasks.md)

## Description
Replace hardcoded field name assumptions with universal key extraction in HashRange query processor to ensure compatibility with schemas using the new universal key format.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 17:40:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 17:45:00 | Status Update | Proposed | InProgress | Started updating HashRange query processor for universal key configuration | ai-agent |
| 2025-09-19 18:00:00 | Status Update | InProgress | Review | Updated HashRange query processor to use universal key configuration, added validation, all tests passing | ai-agent |
| 2025-09-19 18:05:00 | Status Update | Review | Done | Task verified complete - HashRange query processor updated for universal key configuration | ai-agent |

## Requirements
- Replace hardcoded field name assumptions in `src/fold_db_core/query/hash_range_query.rs`
- Use `extract_unified_keys()` to get hash and range field names from schema
- Update `fetch_first_10_hash_keys()` to use universal key configuration
- Modify `query_hashrange_schema()` to extract field names from schema
- Ensure proper hash->range->fields result formatting
- Maintain backward compatibility with existing schemas

## Implementation Plan
1. **Analyze current implementation**: Review `hash_range_query.rs` to identify hardcoded assumptions
2. **Update key extraction**: Replace hardcoded field name assumptions with universal key extraction
3. **Update query methods**: Modify query methods to use schema's key configuration
4. **Add error handling**: Ensure clear error messages for invalid key configurations
5. **Test compatibility**: Verify backward compatibility with existing schemas

## Verification
- HashRange queries work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Query results formatted consistently as hash->range->fields
- Clear error messages for invalid key configurations
- All existing tests pass

## Files Modified
- `src/fold_db_core/query/hash_range_query.rs`
