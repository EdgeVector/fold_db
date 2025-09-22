# PBI-SKC-7: Update Aggregation Utilities for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-7)

## Overview

Update aggregation utilities to use universal key configuration for consistent result formatting, ensuring compatibility with schemas using the new universal key format.

## Problem Statement

The current aggregation utilities (`src/transform/aggregation.rs` and related files) may assume specific field names for result aggregation and formatting. This approach doesn't work with schemas that use the universal key configuration with different field names or when the key configuration uses dotted path expressions.

## User Stories

- **As a developer**, I want aggregation utilities to work with any schema using universal key configuration so I don't have to worry about field name compatibility
- **As a developer**, I want consistent result formatting across all schema types so I can rely on predictable results
- **As a developer**, I want the aggregation utilities to automatically use the correct key field names from the schema so I don't need to hardcode them

## Technical Approach

### 1. Universal Key Integration
- Replace hardcoded field name assumptions with `shape_unified_result()` calls
- Use schema's universal key configuration to determine actual field names
- Support dotted path expressions in key configuration

### 2. Aggregation Updates
- Update aggregation logic to use universal key configuration
- Modify result formatting to handle universal key configurations
- Ensure proper key validation and error handling

### 3. Result Shaping
- Use `shape_unified_result()` for consistent output formatting
- Ensure results are formatted as hash->range->fields across all schema types
- Maintain backward compatibility with existing result formats

## UX/UI Considerations

- No UI changes required (backend-only changes)
- Aggregation results should maintain consistent formatting
- Error messages should be clear and actionable

## Acceptance Criteria

- Aggregation utilities work with schemas using universal key configuration
- Backward compatibility maintained for existing schemas
- Clear error messages for invalid key configurations
- All existing tests pass
- New tests validate universal key functionality
- Aggregation results are consistently formatted as hash->range->fields

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- No external dependencies

## Open Questions

- Should we validate key configuration during aggregation?
- Are there performance implications of dynamic key extraction?

## Notes

- [`docs/design/iterator_stack_quick_reference.md`](../../design/iterator_stack_quick_reference.md) now explains how
  `aggregate_results_unified` consumes universal key metadata and shapes the
  `{ hash, range, fields }` response via `shape_unified_result`, including
  compatibility arrays for legacy range schemas.
- Troubleshooting guidance documents the `SchemaError` surfaced when HashRange
  schemas omit `key.range_field`, aligning with the protections introduced in
  [SKC-1](../SKC-1/prd.md) and exercised by the [SKC-7-2](./SKC-7-2.md) universal
  key test suite.
- Cross-reference [`docs/project_logic.md`](../../project_logic.md) entry
  `SCHEMA-KEY-004` for the enforcement policy covering query, mutation, and
  aggregation components.

## Related Tasks

- Update aggregation utilities implementation
- Add comprehensive tests for universal key scenarios
- Update documentation for aggregation changes
