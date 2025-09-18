# PBI-DTS-MERGE-1: Merge TransformExecutor and StandardizedTransformExecutor

[View in Backlog](../backlog.md#user-content-dts-merge-1)

## Overview

This PBI removes unused executor classes (StandardizedTransformExecutor and OrchestratedTransformExecutor) that are not actually being used in the codebase, keeping only the TransformExecutor that is actively used. This eliminates dead code and reduces architectural complexity without changing any functionality.

## Problem Statement

The declarative transform system currently has multiple executor classes, but analysis shows that only `TransformExecutor` is actually being used:

- **TransformExecutor**: ✅ **Actively Used** - Basic declarative transform execution
- **StandardizedTransformExecutor**: ❌ **Unused** - Three-phase execution pattern (dead code)
- **OrchestratedTransformExecutor**: ❌ **Unused** - Event-driven execution with orchestration (dead code)

This creates several issues:
1. **Dead Code**: Unused executor classes clutter the codebase
2. **Architectural Complexity**: Multiple executor classes that aren't being used
3. **Maintenance Burden**: Dead code requires maintenance without providing value
4. **Confusion**: Developers may wonder which executor to use
5. **Code Review Overhead**: Reviewers must consider unused code

## User Stories

- **As a developer**, I want unused executor classes removed so I can have a cleaner codebase
- **As a developer**, I want to eliminate dead code so I can reduce maintenance overhead
- **As a developer**, I want simplified architecture so I can focus on the executor that's actually being used
- **As a developer**, I want to maintain all existing functionality so I don't break existing transform behavior

## Technical Approach

### 1. Remove Unused Executor Classes

Simply delete the unused executor files and clean up references:

```bash
# Delete unused executor files
rm src/transform/standardized_executor.rs
rm src/transform/orchestrated_executor.rs

# Update module declarations
# Remove from src/transform/mod.rs

# Clean up any unused imports
# Remove any references to deleted executors
```

### 2. Verify No Active Usage

Before deletion, verify that the executors are truly unused:

```bash
# Search for any references to the executors
grep -r "StandardizedTransformExecutor" src/
grep -r "OrchestratedTransformExecutor" src/

# Search for any imports
grep -r "standardized_executor" src/
grep -r "orchestrated_executor" src/
```

## UX/UI Considerations

This PBI is focused on backend cleanup and doesn't require UI changes. The implementation should consider:

- No impact on existing transform execution functionality
- Clean codebase without dead code
- Simplified architecture for future development

## Acceptance Criteria

1. **Dead Code Removal**: Unused executor classes are completely removed
2. **No Functional Changes**: All existing TransformExecutor functionality remains unchanged
3. **Clean Compilation**: Code compiles successfully after removing unused executors
4. **No Broken References**: No remaining imports or references to deleted executors
5. **Maintained Architecture**: TransformExecutor continues to work as before
6. **Clean Module Structure**: Module declarations are updated correctly
7. **Documentation**: Any documentation references to deleted executors are removed

## Dependencies

- Existing TransformExecutor implementation
- Current transform system architecture
- Module system structure

## Open Questions

1. Are there any tests that specifically test the deleted executors that should be removed?
2. Are there any configuration files or documentation that reference the deleted executors?

## Related Tasks

- [DTS-MERGE-1-1: Remove unused executor classes](./DTS-MERGE-1-1.md)

## Implementation Notes

### Files to Delete

- `src/transform/standardized_executor.rs` (if exists)
- `src/transform/orchestrated_executor.rs` (if exists)

### Files to Update

- `src/transform/mod.rs` - Remove module declarations
- Any files with unused imports of deleted executors

### Key Benefits

- **Cleaner Codebase**: Remove dead code that provides no value
- **Reduced Maintenance**: Less code to maintain and review
- **Simplified Architecture**: Focus on the executor that's actually being used
- **No Risk**: No changes to working functionality
