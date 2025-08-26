# [DTS-1-7A] Basic Transform Type Routing

[Back to task list](./tasks.md)

## Description

Implement basic transform type routing to direct procedural and declarative transforms to their appropriate execution paths. This task focuses solely on getting the routing logic working without implementing the actual execution logic.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 17:00:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Transform Type Detection**: Detect whether a transform is procedural or declarative
2. **Basic Routing Logic**: Route transforms to appropriate execution path
3. **Procedural Transform Support**: Ensure existing procedural transforms continue to work
4. **Declarative Transform Placeholder**: Add placeholder for declarative transform execution
5. **Backward Compatibility**: Maintain existing functionality for procedural transforms

## Implementation Plan

### Step 1: Update Transform Executor Routing
- **Modify `execute_transform_with_expr`** in `src/transform/executor.rs`
- **Add transform type detection** using `TransformKind` enum
- **Implement basic routing logic** to direct transforms to appropriate path
- **Maintain existing procedural transform execution** unchanged

### Step 2: Add Declarative Transform Placeholder
- **Add placeholder execution path** for declarative transforms
- **Return simple placeholder result** (e.g., "Declarative transform - not implemented yet")
- **Ensure no errors** when declarative transforms are routed
- **Prepare structure** for future implementation

### Step 3: Update Transform Type Detection
- **Use `Transform::is_declarative()`** method to detect transform type
- **Add logging** to show which execution path is taken
- **Ensure proper error handling** for unknown transform types
- **Validate transform kind** before routing

## Verification

1. **Routing**: Transforms are correctly routed based on their type
2. **Procedural Support**: Existing procedural transforms continue to work unchanged
3. **Declarative Placeholder**: Declarative transforms route to placeholder without errors
4. **Error Handling**: Proper error handling for unknown transform types
5. **Logging**: Clear logging shows which execution path is taken
6. **Backward Compatibility**: No regression in existing procedural transform functionality

## Files Modified

- `src/transform/executor.rs` - Add basic transform type routing logic
- `tests/unit/transform/executor_routing_tests.rs` - Add routing tests

## Test Plan

### Objective
Verify that transform type routing works correctly and procedural transforms continue to function unchanged.

### Test Scope
- Transform type detection and routing
- Procedural transform execution (backward compatibility)
- Declarative transform placeholder routing
- Error handling for unknown transform types

### Environment & Setup
- Standard Rust test environment
- Existing transform system components
- Test fixtures for both transform types

### Mocking Strategy
- Mock external dependencies as needed
- Use existing transform system components for testing
- Create test fixtures for both transform types

### Key Test Scenarios
1. **Procedural Transform Routing**: Test that procedural transforms route to existing execution path
2. **Declarative Transform Routing**: Test that declarative transforms route to placeholder
3. **Transform Type Detection**: Test that transform types are correctly detected
4. **Backward Compatibility**: Test that existing procedural transforms work unchanged
5. **Error Handling**: Test routing with unknown transform types
6. **Logging**: Test that routing decisions are properly logged

### Success Criteria
- All routing tests pass
- Procedural transforms continue to work unchanged
- Declarative transforms route to placeholder without errors
- Transform type detection works correctly
- Proper error handling for unknown types
- Clear logging shows routing decisions
