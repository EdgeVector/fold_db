# [DTS-1-9] Storage and UI Integration Updates

[Back to task list](./tasks.md)

## Description

Update transform storage, display, and logging components to handle both procedural and declarative transform types. This task focuses on ensuring that both transform types can be stored, retrieved, displayed, and logged consistently through the existing infrastructure.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 16:30:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 22:30:00 | Status Update | Proposed | InProgress | Starting storage and UI integration updates | AI Agent |
| 2025-01-27 23:00:00 | Status Update | InProgress | Done | Storage and UI integration updates completed successfully | AI Agent |

## Requirements

1. **Database Operations**: Update serialization/deserialization logic for both transform types
2. **Storage Integration**: Ensure declarative transforms can be stored in existing transform registry
3. **UI Components**: Update display components to show both transform types clearly
4. **Logging Updates**: Add transform type information to log messages and debugging
5. **Backward Compatibility**: Ensure existing procedural transforms continue to work unchanged

## Implementation Plan

### Step 1: Update Database Operations
- **Verify database operations handle both transform types** correctly
- **Update serialization/deserialization logic** to support `TransformKind` enum
- **Ensure backward compatibility** for existing procedural transforms
- **Add migration logic** if needed for database schema updates

### Step 2: Update Transform Storage
- **Update transform registry** to handle both transform types
- **Ensure storage and retrieval** work for both procedural and declarative transforms
- **Add indexes** for efficient declarative transform queries if needed
- **Maintain existing storage patterns** for procedural transforms

### Step 3: Update UI Components
- **Update transform lists** to display transform type information
- **Show declarative schema structure** in detail views
- **Add validation feedback** for declarative transforms
- **Ensure error messages** are clear for both transform types

### Step 4: Update Logging and Debugging
- **Add transform type to log messages** for better debugging
- **Include declarative schema details** in debug logs
- **Add performance metrics** for both transform types
- **Ensure consistent logging** across all transform operations

### Step 5: Add Debugging Information
- **Implement `get_debug_info`** method for transforms
- **Provide clear information** about transform type and structure
- **Add debugging helpers** for declarative transform inspection
- **Ensure debugging tools** work for both transform types

## Verification

1. **Storage**: Both transform types can be stored and retrieved correctly
2. **UI Display**: Transform type information is clearly displayed
3. **Logging**: Transform type information appears in logs and debugging
4. **Backward Compatibility**: Existing procedural transforms work unchanged
5. **Performance**: No performance degradation from new functionality
6. **User Experience**: Clear information about transform types and validation

## Files Modified

**Implementation Files:**
- `src/db_operations/transform_operations.rs` - Enhanced database operations with transform type logging for storage and retrieval
- `src/fold_db_core/transform_manager/manager.rs` - Updated transform manager to include transform type information in loading logs
- `src/fold_db_core/transform_manager/loading.rs` - Enhanced transform registration logging with transform type information
- `src/schema/types/transform.rs` - Added `get_debug_info()` method for comprehensive transform debugging information
- `src/datafold_node/static-react/src/components/tabs/TransformsTab.jsx` - Updated UI to display transform type information and handle both procedural and declarative transforms

**Test Files:**
- `tests/integration/storage_integration_tests.rs` - Comprehensive storage integration tests (6 tests) covering both transform types, transform manager integration, backward compatibility, debug information, error handling, and performance
- `tests/integration/mod.rs` - Added storage_integration_tests module

**Key Features Implemented:**
- Enhanced database operations with transform type logging for better debugging and monitoring
- Transform manager integration with transform type information in all logging operations
- UI components updated to clearly display transform type (Procedural vs Declarative) with appropriate styling
- Comprehensive debug information method for transforms including type, inputs, and output details
- Full backward compatibility maintained for existing procedural transforms
- Comprehensive test coverage for storage operations, transform manager integration, and UI display

## Test Plan

### Objective
Verify that both transform types can be stored, retrieved, displayed, and logged correctly through the existing infrastructure without breaking existing functionality.

### Test Scope
- Database operations for both transform types
- Storage and retrieval integration
- UI display updates
- Logging and debugging enhancements
- Backward compatibility verification

### Environment & Setup
- Standard Rust test environment
- Existing transform system components
- Test database with both transform types
- UI components (if applicable)
- Logging infrastructure

### Mocking Strategy
- Mock external dependencies as needed
- Use existing transform system components for integration testing
- Create test fixtures for both transform types
- Mock database operations for testing storage

### Key Test Scenarios
1. **Storage Integration**: Test storing and retrieving both transform types
2. **UI Display**: Test that transform type information is displayed correctly
3. **Logging Updates**: Test that transform type appears in logs
4. **Debugging Information**: Test debugging tools for both transform types
5. **Backward Compatibility**: Test existing procedural transforms still work
6. **Performance Testing**: Test storage and retrieval performance
7. **Error Handling**: Test error scenarios for both transform types

### Success Criteria
- All storage integration tests pass
- Both transform types can be stored and retrieved correctly
- UI components display transform type information clearly
- Logging includes transform type information
- Backward compatibility is maintained
- Performance is acceptable for both transform types
- Error handling works properly for both transform types
