# [DTS-CONSOLIDATE-1-3] Delete separate executor modules and update imports

[Back to task list](./tasks.md)

## Description

Delete the three separate executor modules (`single_executor.rs`, `range_executor.rs`, `hash_range_executor.rs`) and update all imports and module declarations to use the unified execution pattern in `executor.rs`. This completes the consolidation by removing the duplicate code.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-27 20:00:00 | Created | N/A | Proposed | Task file created | AI Agent |

## Requirements

1. **Delete Executor Files**: Remove the three separate executor module files
2. **Update Module Declarations**: Remove module declarations from `mod.rs`
3. **Update Imports**: Update any remaining imports that reference the deleted modules
4. **Clean Up References**: Remove any references to the deleted executor modules
5. **Verify Compilation**: Ensure code compiles after cleanup

## Implementation Plan

### Step 1: Delete Executor Module Files

1. **Delete Files**:
   ```bash
   rm src/transform/single_executor.rs
   rm src/transform/range_executor.rs  
   rm src/transform/hash_range_executor.rs
   ```

2. **Verify Deletion**: Confirm files are completely removed from the filesystem

### Step 2: Update Module Declarations

1. **Update `src/transform/mod.rs`**: Remove module declarations:
   ```rust
   // Remove these lines:
   pub mod single_executor;
   pub mod range_executor;
   pub mod hash_range_executor;
   ```

2. **Update Re-exports**: Remove any re-exports of the deleted modules:
   ```rust
   // Remove any re-exports like:
   // pub use single_executor::*;
   // pub use range_executor::*;
   // pub use hash_range_executor::*;
   ```

### Step 3: Search for Remaining References

1. **Search for Imports**: Find any remaining imports of the deleted modules:
   ```bash
   grep -r "single_executor\|range_executor\|hash_range_executor" src/
   ```

2. **Search for Direct Function Calls**: Find any direct calls to functions from deleted modules:
   ```bash
   grep -r "execute_single_schema\|execute_range_schema\|execute_hashrange_schema" src/
   ```

3. **Search for Test References**: Check for test files that might reference the deleted modules:
   ```bash
   grep -r "single_executor\|range_executor\|hash_range_executor" tests/
   ```

### Step 4: Clean Up Remaining References

1. **Update Direct Function Calls**: If any code directly calls functions from deleted modules, update to use the unified executor:
   ```rust
   // Change from:
   // single_executor::execute_single_schema(schema, input_values)
   
   // To:
   // TransformExecutor::execute_declarative_transform_unified(schema, input_values)
   ```

2. **Update Import Statements**: Replace any imports of the deleted modules:
   ```rust
   // Remove:
   // use crate::transform::single_executor;
   // use crate::transform::range_executor;
   // use crate::transform::hash_range_executor;
   
   // Keep only:
   // use crate::transform::executor::TransformExecutor;
   ```

3. **Update Test Files**: Update any test files that reference the deleted modules:
   ```rust
   // Update test imports and function calls to use TransformExecutor
   ```

### Step 5: Verify Compilation

1. **Check Compilation**: Ensure code compiles after cleanup:
   ```bash
   cargo check --workspace
   ```

2. **Run Tests**: Verify all tests still pass:
   ```bash
   cargo test --workspace
   ```

3. **Check for Warnings**: Look for any compilation warnings about unused imports or dead code

### Step 6: Final Cleanup

1. **Remove Unused Imports**: Clean up any unused imports that were only used by deleted modules
2. **Update Documentation**: Update any documentation that references the deleted modules
3. **Verify Module Structure**: Ensure the transform module structure is clean and consistent

## Test Plan

### Objective
Verify that deleting the separate executor modules doesn't break compilation or functionality.

### Test Scope
- Compilation success
- All existing tests pass
- No broken references
- Clean module structure

### Key Test Scenarios
1. **Compilation Test**: Ensure code compiles after deleting modules
2. **Test Suite**: Verify all tests pass without modification
3. **Import Verification**: Confirm no broken imports remain
4. **Functionality**: Ensure transform execution still works
5. **Module Structure**: Verify clean module organization

### Success Criteria
- Code compiles successfully
- All tests pass
- No broken references or imports
- Clean module structure
- No compilation warnings about missing modules

## Files Modified

- `src/transform/mod.rs` - Remove module declarations
- Any files with imports of deleted modules - Update imports
- Any test files referencing deleted modules - Update references

## Files Deleted

- `src/transform/single_executor.rs` - Merged into executor.rs
- `src/transform/range_executor.rs` - Merged into executor.rs
- `src/transform/hash_range_executor.rs` - Merged into executor.rs

## Verification

1. **File Deletion**: All three executor module files are completely removed
2. **Module Declarations**: Module declarations removed from mod.rs
3. **Import Cleanup**: All imports of deleted modules are removed or updated
4. **Compilation**: Code compiles successfully after cleanup
5. **Test Suite**: All tests pass without modification
6. **No Broken References**: No remaining references to deleted modules
7. **Clean Structure**: Module structure is clean and consistent
