# PBI-SKC-3: Update Mutation Processor for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-3)

## Overview

Update the mutation processor to use universal key extraction instead of hardcoded `hash_key` and `range_key` field assumptions, ensuring compatibility with schemas using the new universal key format.

## Problem Statement

The current mutation processor (`src/fold_db_core/mutation/mutation_processor.rs`) hardcodes the extraction of `hash_key` and `range_key` fields from mutation data. This approach doesn't work with schemas that use the universal key configuration with different field names or when the key configuration is optional.

## User Stories

- **As a developer**, I want mutations to work with any schema using universal key configuration so I don't have to worry about field name compatibility
- **As a developer**, I want the mutation processor to automatically determine the correct key field names from the schema so I don't need to hardcode them
- **As a developer**, I want consistent mutation behavior across all schema types so I can rely on predictable results

## Technical Approach

### 1. Universal Key Integration
- Replace hardcoded `hash_key` and `range_key` field extraction with `extract_unified_keys()` calls
- Use schema's universal key configuration to determine actual field names
- Handle optional key configurations for Single and Range schemas

### 2. Mutation Processing Updates
- Update `process_field_mutations_via_service()` to use universal key extraction
- Modify HashRange mutation handling to extract keys from schema configuration
- Ensure proper key validation and error handling

### 3. Backward Compatibility
- Maintain support for existing mutation formats
- Handle schemas without universal key configuration gracefully
- Preserve existing mutation behavior for legacy schemas

## UX/UI Considerations

- No UI changes required (backend-only changes)
- Mutation results should maintain consistent formatting
- Error messages should be clear and actionable

## Acceptance Criteria

- Mutations work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Clear error messages for invalid key configurations
- All existing tests pass
- New tests validate universal key functionality
- Mutation processing handles optional key configurations correctly

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- No external dependencies

## Open Questions

- Should we validate key configuration during mutation processing?
- Are there performance implications of dynamic key extraction?

## Related Tasks

- Update mutation processor implementation
- Add comprehensive tests for universal key scenarios
- Update documentation for mutation processor changes
