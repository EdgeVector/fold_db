# PBI-SKC-4: Update Mutation Service for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-4)

## Overview

Update the mutation service to use universal key configuration for HashRange schemas instead of assuming specific `hash_key` and `range_key` field names, ensuring compatibility with schemas using the new universal key format.

## Problem Statement

The current mutation service (`src/fold_db_core/services/mutation.rs`) assumes that HashRange schemas will have `hash_key` and `range_key` fields in the mutation data. This approach doesn't work with schemas that use the universal key configuration with different field names or when the key configuration uses dotted path expressions.

## User Stories

- **As a developer**, I want HashRange mutations to work with any schema using universal key configuration so I don't have to worry about field name compatibility
- **As a developer**, I want the mutation service to automatically extract the correct key field names from the schema so I don't need to hardcode them
- **As a developer**, I want consistent mutation behavior across all schema types so I can rely on predictable results

## Technical Approach

### 1. Universal Key Integration
- Replace hardcoded `hash_key` and `range_key` field assumptions with universal key extraction
- Use schema's universal key configuration to determine actual field names
- Support dotted path expressions in key configuration

### 2. Mutation Service Updates
- Update `update_hashrange_schema_fields()` to use universal key extraction
- Modify HashRange field processing to extract keys from schema configuration
- Ensure proper key validation and error handling

### 3. Field Processing
- Update field processing logic to handle universal key configurations
- Support both direct field access and dotted path expressions
- Maintain backward compatibility with existing mutation formats

## UX/UI Considerations

- No UI changes required (backend-only changes)
- Mutation results should maintain consistent formatting
- Error messages should be clear and actionable

## Acceptance Criteria

- HashRange mutations work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Clear error messages for invalid key configurations
- All existing tests pass
- New tests validate universal key functionality
- Mutation service handles dotted path expressions correctly

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- Depends on SKC-3 (Mutation Processor) completion
- No external dependencies

## Open Questions

- Should we validate key configuration during mutation processing?
- Are there performance implications of dynamic key extraction?

## Related Tasks

- Update mutation service implementation
- Add comprehensive tests for universal key scenarios
- Update documentation for mutation service changes
