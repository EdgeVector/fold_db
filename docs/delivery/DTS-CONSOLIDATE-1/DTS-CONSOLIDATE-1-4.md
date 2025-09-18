# [DTS-CONSOLIDATE-1-4] Update tests and verify functionality preservation

[Back to task list](./tasks.md)

## Description

Update any tests that reference the deleted executor modules and verify that all existing functionality is preserved after the consolidation. Ensure comprehensive test coverage and validate that the unified execution pattern works identically to the separate executors.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 20:00:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Update Test Imports**: Update any test files that import the deleted executor modules
2. **Verify Test Coverage**: Ensure all existing test scenarios are still covered
3. **Functionality Validation**: Verify that all transform execution functionality works identically
4. **Performance Validation**: Ensure no performance degradation
5. **Integration Testing**: Verify integration with the broader transform system

## Implementation Plan

### Step 1: Identify Test Dependencies

1. **Search for Test References**: Find all test files that reference the deleted modules:
   ```bash
   grep -r "single_executor\|range_executor\|hash_range_executor" tests/
   grep -r "execute_single_schema\|execute_range_schema\|execute_hashrange_schema" tests/
   ```

2. **Identify Test Scenarios**: Document all test scenarios that need to be preserved:
   - Single schema execution tests
   - Range schema execution tests
   - HashRange schema execution tests
   - Error handling tests
   - Integration tests

### Step 2: Update Test Imports

1. **Update Import Statements**: Replace imports of deleted modules with unified executor:
   ```rust
   // Change from:
   // use crate::transform::single_executor::execute_single_schema;
   // use crate::transform::range_executor::execute_range_schema;
   // use crate::transform::hash_range_executor::execute_hashrange_schema;
   
   // To:
   // use crate::transform::executor::TransformExecutor;
   ```

2. **Update Function Calls**: Replace direct function calls with unified executor calls:
   ```rust
   // Change from:
   // let result = execute_single_schema(&schema, input_values);
   
   // To:
   // let result = TransformExecutor::execute_declarative_transform_unified(&schema, input_values);
   ```

### Step 3: Preserve Test Scenarios

1. **Single Schema Tests**: Ensure all Single schema test scenarios are preserved:
   ```rust
   #[test]
   fn test_execute_single_schema_simple() {
       // Test simple Single schema execution
       let result = TransformExecutor::execute_declarative_transform_unified(&schema, input_values);
       // Verify identical behavior to original single_executor
   }
   
   #[test]
   fn test_execute_single_schema_missing_field() {
       // Test handling of missing input fields
   }
   
   #[test]
   fn test_execute_single_schema_complex_expression() {
       // Test handling of complex expressions
   }
   ```

2. **Range Schema Tests**: Ensure all Range schema test scenarios are preserved:
   ```rust
   #[test]
   fn test_execute_range_schema_basic() {
       // Test basic Range schema execution
   }
   
   #[test]
   fn test_execute_range_schema_with_range_key() {
       // Test Range schema with specific range key
   }
   ```

3. **HashRange Schema Tests**: Ensure all HashRange schema test scenarios are preserved:
   ```rust
   #[test]
   fn test_execute_hashrange_schema_basic() {
       // Test basic HashRange schema execution
   }
   
   #[test]
   fn test_execute_hashrange_schema_with_key_config() {
       // Test HashRange schema with key configuration
   }
   ```

### Step 4: Add Comprehensive Integration Tests

1. **End-to-End Tests**: Add tests that verify the complete execution flow:
   ```rust
   #[test]
   fn test_unified_executor_all_schema_types() {
       // Test all three schema types with the unified executor
       let single_schema = create_test_single_schema();
       let range_schema = create_test_range_schema();
       let hashrange_schema = create_test_hashrange_schema();
       
       // Execute all and verify results
       let single_result = TransformExecutor::execute_declarative_transform_unified(&single_schema, input_values.clone());
       let range_result = TransformExecutor::execute_declarative_transform_unified(&range_schema, input_values.clone());
       let hashrange_result = TransformExecutor::execute_declarative_transform_unified(&hashrange_schema, input_values.clone());
       
       // Verify all results are correct
       assert!(single_result.is_ok());
       assert!(range_result.is_ok());
       assert!(hashrange_result.is_ok());
   }
   ```

2. **Error Handling Tests**: Verify error handling behavior is preserved:
   ```rust
   #[test]
   fn test_unified_executor_error_handling() {
       // Test error handling for invalid schemas
       // Test error handling for missing input values
       // Test error handling for parsing errors
   }
   ```

3. **Performance Tests**: Verify no performance degradation:
   ```rust
   #[test]
   fn test_unified_executor_performance() {
       // Benchmark unified executor performance
       // Compare with expected performance characteristics
   }
   ```

### Step 5: Validation Testing

1. **Functionality Comparison**: Create tests that compare behavior between old and new executors:
   ```rust
   #[test]
   fn test_executor_behavior_preservation() {
       // Create identical test cases for each schema type
       // Verify results are identical between old and new implementations
       // This test can be removed after validation is complete
   }
   ```

2. **Regression Testing**: Run comprehensive regression tests:
   ```bash
   cargo test --workspace --release
   cargo test transform --release
   cargo test integration --release
   ```

3. **Integration Testing**: Verify integration with the broader system:
   ```rust
   #[test]
   fn test_transform_executor_integration() {
       // Test integration with TransformExecutor public API
       // Test integration with transform system
       // Test integration with schema system
   }
   ```

## Test Plan

### Objective
Verify that all existing functionality is preserved and that the unified execution pattern works correctly.

### Test Scope
- All existing executor functionality
- New unified execution pattern
- Integration with transform system
- Performance characteristics
- Error handling behavior

### Key Test Scenarios
1. **Single Schema Execution**: All Single schema tests pass with unified executor
2. **Range Schema Execution**: All Range schema tests pass with unified executor
3. **HashRange Schema Execution**: All HashRange schema tests pass with unified executor
4. **Error Handling**: Error handling behavior is preserved
5. **Performance**: No performance degradation
6. **Integration**: Integration with broader system works correctly
7. **Regression**: All existing tests continue to pass

### Success Criteria
- All existing tests pass without modification
- New unified executor tests pass
- Performance benchmarks show no degradation
- Error handling behavior is identical
- Integration tests pass
- Code coverage is maintained or improved

## Files Modified

- Test files that reference deleted executor modules - Update imports and function calls
- `src/transform/executor.rs` - Add any missing tests
- Integration test files - Update to use unified executor

## Verification

1. **Test Updates**: All test files updated to use unified executor
2. **Test Coverage**: All existing test scenarios are preserved
3. **Test Execution**: All tests pass with unified executor
4. **Functionality**: All transform execution functionality works identically
5. **Performance**: No performance degradation detected
6. **Integration**: Integration with broader system works correctly
7. **Regression**: No regressions introduced by consolidation
