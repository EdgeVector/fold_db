# [SKC-3-1] Update mutation processor to use universal key extraction

[Back to task list](./tasks.md)

## Description
Replace hardcoded field name assumptions with universal key extraction in mutation processor to ensure compatibility with schemas using the new universal key format.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 19:10:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 19:15:00 | Status Update | Proposed | InProgress | Started updating mutation processor for universal key configuration | ai-agent |
| 2025-09-19 19:45:00 | Status Update | InProgress | Review | Updated mutation processor to use universal key extraction, cleaned up debug statements, all tests passing | ai-agent |
| 2025-09-19 19:50:00 | Status Update | Review | Done | Task verified complete - mutation processor updated for universal key configuration | ai-agent |

## Requirements
- Replace hardcoded `"hash_key"` and `"range_key"` field name assumptions in mutation processor
- Use `extract_unified_keys()` to get hash and range field names from schema
- Update HashRange mutation processing to use universal key configuration
- Update Range mutation processing to use universal key configuration
- Remove debug println! statements and replace with proper logging
- Maintain backward compatibility with existing mutations
- Ensure clear error messages for invalid key configurations
- All tests must pass

## Implementation Plan
1. **Analyze current implementation**: Review mutation processor to identify hardcoded assumptions
2. **Update key extraction**: Replace hardcoded field names with universal key extraction
3. **Update HashRange processing**: Use universal key configuration for HashRange mutations
4. **Update Range processing**: Use universal key configuration for Range mutations
5. **Clean up debug statements**: Remove println! and replace with proper logging
6. **Test compatibility**: Verify backward compatibility with existing mutations

## Verification
- Mutation processor works with schemas using universal key configuration
- HashRange mutations extract keys from universal key configuration
- Range mutations extract keys from universal key configuration
- Backward compatibility maintained for existing mutations
- Clear error messages for invalid key configurations
- All tests pass
- No clippy issues

## Files Modified
- `src/fold_db_core/mutation/mutation_processor.rs` (update key extraction logic)
- `src/fold_db_core/services/mutation.rs` (update mutation service methods)
