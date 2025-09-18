# [DTS-MERGE-1-1] Remove unused executor classes

[Back to task list](./tasks.md)

## Description

Remove the unused `StandardizedTransformExecutor` and `OrchestratedTransformExecutor` classes, keeping only the `TransformExecutor` that is actually being used in the codebase. This eliminates dead code and reduces architectural complexity without changing any functionality.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 19:00:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-27 19:30:00 | Status Update | Proposed | InProgress | Started implementation | AI Agent |
| 2025-01-27 19:45:00 | Status Update | InProgress | Review | Implementation completed, all tests pass | AI Agent |
| 2025-01-27 19:50:00 | Status Update | Review | Done | Changes committed successfully | AI Agent |

## Requirements

1. **Delete Unused Files**: Remove StandardizedTransformExecutor and OrchestratedTransformExecutor files
2. **Update Module Declarations**: Remove from src/transform/mod.rs
3. **Clean Up Imports**: Remove any unused imports of the deleted executors
4. **Verify No Usage**: Ensure no code is actually using the deleted executors
5. **Maintain Existing Functionality**: Keep TransformExecutor unchanged

## Implementation Plan

### Step 1: Verify No Active Usage

1. **Search for Usage**: Use grep to find all references to StandardizedTransformExecutor and OrchestratedTransformExecutor
2. **Confirm Unused**: Verify these executors are not actually being used in the codebase
3. **Document Findings**: List any references found and confirm they can be safely removed

### Step 2: Remove Unused Executor Files

1. **Delete Files**:
   - `src/transform/standardized_executor.rs` (if exists)
   - `src/transform/orchestrated_executor.rs` (if exists)

2. **Update Module Declarations**:
   - Remove from `src/transform/mod.rs`
   - Clean up any unused imports

### Step 3: Clean Up References

1. **Remove Unused Imports**: Find and remove any imports of the deleted executors
2. **Remove Unused Tests**: Delete any tests that were testing the removed executors
3. **Update Documentation**: Remove any documentation references to the deleted executors

## Test Plan

### Objective
Verify that removing unused executor classes doesn't break existing functionality.

### Test Scope
- Existing TransformExecutor functionality remains unchanged
- Code compiles successfully after removing unused executors
- No runtime errors from missing executor references

### Key Test Scenarios
1. **Compilation Test**: Ensure code compiles after removing unused executors
2. **Existing Functionality**: Verify TransformExecutor still works as before
3. **Integration Tests**: Ensure orchestration integration still works with TransformExecutor
4. **No Dead Code**: Confirm no unused imports or references remain

### Success Criteria
- Code compiles without errors
- All existing tests pass
- No regressions in transform execution
- Clean codebase with no unused executor references

## Files Modified

- `src/transform/standardized_executor.rs` - **DELETED**
- `src/transform/orchestrated_executor.rs` - **DELETED**
- `src/transform/mod.rs` - Updated module declarations
- Any files with unused imports of removed executors - Cleaned up

## Verification

1. **Compilation Test**: Ensure code compiles after removing unused executors
2. **Existing Tests**: Verify all existing tests still pass
3. **Integration Tests**: Ensure orchestration integration still works
4. **Code Review**: Verify no unused imports or dead code remains
5. **Documentation**: Ensure no references to deleted executors remain
