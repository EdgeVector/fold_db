# PBI-SKC-6: Update Field Processing Utilities for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-6)

## Overview

Update field processing utilities to use universal key extraction instead of hardcoded field name assumptions, ensuring compatibility with schemas using the new universal key format.

## Problem Statement

The current field processing utilities (`src/fold_db_core/managers/atom/field_processing.rs` and related files) may have hardcoded assumptions about field names for key extraction and processing. This approach doesn't work with schemas that use the universal key configuration with different field names or when the key configuration uses dotted path expressions.

## User Stories

- **As a developer**, I want field processing utilities to work with any schema using universal key configuration so I don't have to worry about field name compatibility
- **As a developer**, I want consistent field processing behavior across all schema types so I can rely on predictable results
- **As a developer**, I want the field processing utilities to automatically extract the correct key field names from the schema so I don't need to hardcode them

## Technical Approach

### 1. Universal Key Integration
- Replace hardcoded field name assumptions with `extract_unified_keys()` calls
- Use schema's universal key configuration to determine actual field names
- Support dotted path expressions in key configuration

### 2. Field Processing Updates
- Update field processing logic to use universal key extraction
- Modify atom processing to handle universal key configurations
- Ensure proper key validation and error handling

### 3. Utility Function Updates
- Update utility functions to use universal key extraction
- Ensure consistent behavior across all field processing operations
- Maintain backward compatibility with existing schemas

## UX/UI Considerations

- No UI changes required (backend-only changes)
- Field processing results should maintain consistent formatting
- Error messages should be clear and actionable

## Acceptance Criteria

- Field processing utilities work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Clear error messages for invalid key configurations
- All existing tests pass
- New tests validate universal key functionality
- Field processing handles dotted path expressions correctly
- Documentation links to the [migration workflow](../../universal-key-migration-guide.md#field-processing-and-mutation-workflow)
  and the reference pages for the [MutationService](../../reference/fold_db_core/mutation_service.md) and
  [AtomManager field processing](../../reference/fold_db_core/field_processing.md)

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- No external dependencies

## Open Questions

- Should we validate key configuration during field processing?
- Are there performance implications of dynamic key extraction?

## Related Tasks

- Update field processing utilities implementation
- Add comprehensive tests for universal key scenarios
- Update documentation for field processing changes
