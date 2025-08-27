# [DTS-1-7B] Simple Declarative Transform Execution

[Back to task list](./tasks.md)

## Description

Implement basic execution for simple declarative transforms, focusing on "Single" schema type with basic field resolution. This task builds on the routing foundation from DTS-1-7A to provide actual execution capability for the simplest declarative transform case.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 17:00:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 20:30:00 | Status Update | Proposed | InProgress | Started implementation | AI Agent |
| 2025-01-27 21:15:00 | Status Update | InProgress | Done | Single schema execution implemented and tested | AI Agent |

## Requirements

1. **Single Schema Support**: Handle "Single" schema type execution
2. **Basic Field Resolution**: Resolve field values from source data
3. **Simple Result Generation**: Generate basic result objects
4. **Error Handling**: Basic error handling for malformed schemas
5. **Integration**: Integrate with routing from DTS-1-7A

## Dependencies

- **DTS-1-7A**: Basic Transform Type Routing (must be completed first)
- **DTS-1-6**: Schema Interpreter (for parsing declarative transforms)
- **DTS-1-1**: TransformKind enum (for transform type detection)
- **DTS-1-2**: DeclarativeSchemaDefinition (for schema structure)

## Implementation Plan

### Step 1: Implement Single Schema Execution
- **Add `execute_single_schema` method** to `TransformExecutor`
- **Handle "Single" schema type** in execution routing
- **Basic field iteration** through schema fields
- **Simple result object creation**

### Step 2: Basic Field Resolution
- **Implement simple field resolution** for atom UUID expressions
- **Basic source data lookup** without complex parsing
- **Handle missing fields gracefully** (return null/default values)
- **Simple error handling** for resolution failures

### Step 3: Result Generation
- **Create result objects** with resolved field values
- **Handle different field types** (strings, numbers, objects)
- **Basic validation** of result structure
- **Return properly formatted JSON results**

### Step 4: Error Handling and Validation
- **Validate schema structure** before execution
- **Handle missing required fields** gracefully
- **Provide clear error messages** for common issues
- **Ensure execution doesn't crash** on malformed schemas

## Verification

1. **Single Schema Execution**: "Single" schema type executes correctly
2. **Field Resolution**: Basic field values are resolved from source data
3. **Result Generation**: Proper result objects are generated
4. **Error Handling**: Malformed schemas are handled gracefully
5. **Integration**: Works with routing from DTS-1-7A
6. **Backward Compatibility**: Procedural transforms continue to work

## Files Modified

- `src/transform/executor.rs` - Added single schema execution logic with field resolution
- `tests/unit/transform/single_schema_execution_tests.rs` - Added comprehensive execution tests
- `tests/unit/transform/executor_routing_tests.rs` - Updated routing tests for new execution behavior  
- `tests/unit/transform/mod.rs` - Added new test module inclusion

## Test Plan

### Objective
Verify that simple declarative transforms with "Single" schema type can execute correctly and generate proper results.

### Test Scope
- Single schema execution
- Basic field resolution
- Result generation
- Error handling for malformed schemas
- Integration with routing system

### Environment & Setup
- Standard Rust test environment
- Existing transform system components
- Test fixtures for single schema transforms
- Completed DTS-1-7A (routing foundation)

### Mocking Strategy
- Mock external dependencies as needed
- Use existing transform system components for testing
- Create test fixtures for single schema scenarios

### Key Test Scenarios
1. **Valid Single Schema**: Test execution of valid single schema transforms
2. **Field Resolution**: Test that field values are correctly resolved
3. **Result Generation**: Test that proper result objects are created
4. **Missing Fields**: Test handling of missing or undefined fields
5. **Malformed Schema**: Test error handling for invalid schema structures
6. **Integration**: Test integration with routing system from DTS-1-7A

### Success Criteria
- All single schema execution tests pass
- Field values are correctly resolved from source data
- Result objects are properly generated
- Error handling works for malformed schemas
- Integration with routing system works correctly
- No regression in procedural transform functionality
