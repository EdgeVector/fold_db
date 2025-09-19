# PBI-SKC-5: Update Transform Tests for Universal Key Configuration

[View in Backlog](../backlog.md#user-content-SKC-5)

## Overview

Update transform tests to use universal key configuration instead of legacy `range_key` patterns, ensuring comprehensive test coverage for the new universal key functionality.

## Problem Statement

The current transform tests (`tests/unit/transform/range_schema_tests.rs` and related files) still use the legacy `SchemaType::Range { range_key }` pattern without universal key configuration. This means the tests don't validate the new universal key functionality and may not catch regressions in the universal key system.

## User Stories

- **As a developer**, I want transform tests to validate universal key functionality so I can be confident the system works correctly
- **As a developer**, I want comprehensive test coverage for all schema types using universal key configuration so I can catch regressions early
- **As a developer**, I want tests that demonstrate the correct usage of universal key configuration so I can understand how to use it

## Technical Approach

### 1. Test Schema Updates
- Update test schemas to use universal key configuration
- Replace legacy `SchemaType::Range { range_key }` with universal key format
- Add test cases for Single, Range, and HashRange schemas with universal keys

### 2. Test Coverage Expansion
- Add tests for universal key extraction functionality
- Test backward compatibility with legacy schemas
- Validate error handling for invalid key configurations

### 3. Test Data Updates
- Update test data to work with universal key configuration
- Ensure test scenarios cover all key configuration scenarios
- Add edge cases for optional key configurations

## UX/UI Considerations

- No UI changes required (test-only changes)
- Tests should validate both new and legacy functionality
- Test output should be clear and actionable

## Acceptance Criteria

- All transform tests use universal key configuration
- Comprehensive test coverage for universal key functionality
- Backward compatibility tests for legacy schemas
- Clear test failures for invalid key configurations
- All tests pass with universal key configuration
- Test documentation updated to reflect changes

## Dependencies

- Depends on SKC-1 (Universal Key Configuration) completion
- No external dependencies

## Open Questions

- Should we add performance tests for universal key extraction?
- Are there specific edge cases we should test?

## Related Tasks

- Update transform test schemas and data
- Add comprehensive universal key test cases
- Update test documentation
- Validate test coverage for universal key functionality
