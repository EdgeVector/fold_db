# PBI-SKC-2: Update HashRange Query Processor for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-2)

## Overview

Update the HashRange query processor to use the universal key configuration instead of hardcoded field name assumptions, ensuring compatibility with schemas that use the new universal key format.

## Problem Statement

The current HashRange query processor (`src/fold_db_core/query/hash_range_query.rs`) makes hardcoded assumptions about field names for hash key extraction and query processing. This approach doesn't work with schemas that use the universal key configuration with different field names.

## User Stories

- **As a developer**, I want HashRange queries to work with any schema using universal key configuration so I don't have to worry about field name compatibility
- **As a developer**, I want consistent query behavior across all schema types so I can rely on predictable results
- **As a developer**, I want the query processor to automatically extract the correct hash and range field names from the schema so I don't need to hardcode them

## Technical Approach

### 1. Universal Key Integration
- Replace hardcoded field name assumptions with `extract_unified_keys()` calls
- Use schema's universal key configuration to determine actual field names
- Maintain backward compatibility with existing schemas

### 2. Query Processing Updates
- Update `fetch_first_10_hash_keys()` to use universal key configuration
- Modify `query_hashrange_schema()` to extract field names from schema
- Ensure proper hash->range->fields result formatting

### 3. Error Handling
- Add validation for missing key configuration in HashRange schemas
- Provide clear error messages when key configuration is invalid
- Handle edge cases gracefully

## UX/UI Considerations

- No UI changes required (backend-only changes)
- Query results should maintain consistent formatting
- Error messages should be clear and actionable

## Acceptance Criteria

- HashRange queries work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Query results formatted consistently as hash->range->fields
- Clear error messages for invalid key configurations
- All existing tests pass
- New tests validate universal key functionality

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- No external dependencies

## Open Questions

- Should we deprecate any existing query methods?
- Are there performance implications of dynamic field name extraction?

## Related Tasks

- Update HashRange query processor implementation
- Add comprehensive tests for universal key scenarios
- Update documentation for query processor changes
