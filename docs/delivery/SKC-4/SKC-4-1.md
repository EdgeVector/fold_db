# [SKC-4-1] Update mutation service to use universal key configuration

[Back to task list](./tasks.md)

## Description
Update the mutation service to use universal key configuration instead of hardcoded field name assumptions, ensuring HashRange mutations work with any schema using the unified key system.

## Status History
| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-09-19 21:30:00 | Created | N/A | Proposed | Task file created | ai-agent |
| 2025-09-19 21:35:00 | Status Update | Proposed | InProgress | Started updating mutation service for universal key configuration | ai-agent |
| 2025-09-19 21:45:00 | Status Update | InProgress | Review | Mutation service updated to use universal key configuration, all tests passing | ai-agent |

## Requirements
- Update mutation service to use universal key configuration for HashRange schemas
- Replace hardcoded field name assumptions with dynamic key extraction
- Maintain backward compatibility with existing mutation patterns
- Provide clear error messages for invalid key configurations
- Ensure all mutation service methods work with universal key configuration
- Update field processing logic to use universal key extraction
- Maintain existing functionality while adding universal key support

## Implementation Plan
1. **Analyze current mutation service**: Review existing HashRange mutation handling
2. **Update key extraction**: Replace hardcoded field names with universal key configuration
3. **Update field processing**: Modify field processing logic to use universal key extraction
4. **Update error handling**: Provide clear error messages for invalid key configurations
5. **Maintain backward compatibility**: Ensure existing mutations continue to work
6. **Test changes**: Verify all functionality works with universal key configuration

## Verification
- Mutation service uses universal key configuration for HashRange schemas
- Backward compatibility maintained for existing mutations
- Clear error messages for invalid key configurations
- All existing functionality preserved
- Code compiles without errors
- All tests pass

## Files Modified
- `src/fold_db_core/services/mutation.rs` (update mutation service logic)
