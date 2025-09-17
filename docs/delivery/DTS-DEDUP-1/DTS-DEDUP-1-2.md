# DTS-DEDUP-1-2 Consolidate Expression Parsing

[Back to task list](./tasks.md)

## Description

Eliminate duplicate expression parsing logic across executor modules by expanding shared utilities. This task addresses the second most significant duplication where identical expression parsing patterns are repeated across multiple executor modules.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-01-17 15:45:00 | Created | N/A | Proposed | Task file created | AI Agent |
| 2025-01-17 16:35:00 | Status Update | Proposed | InProgress | Started work on expression parsing consolidation | AI Agent |
| 2025-01-17 17:00:00 | Status Update | InProgress | Review | Completed expression parsing consolidation with unified functions and refactored all executor modules | AI Agent |

## Requirements

### Functional Requirements
1. **Consolidate Parsing Logic**: Create unified expression parsing functions
2. **Remove Duplicates**: Eliminate duplicate parsing from executor modules
3. **Preserve Behavior**: Maintain all existing parsing behavior
4. **Maintain Performance**: Ensure no performance regression

### Technical Requirements
1. **Shared Utilities**: Expand `src/transform/shared_utilities.rs` with parsing functions
2. **Batch Processing**: Create `parse_expressions_batch()` function
3. **Schema Collection**: Create `collect_expressions_from_schema()` function
4. **Error Handling**: Unified error handling for parsing failures

### Quality Requirements
1. **Test Coverage**: Maintain >90% test coverage
2. **Documentation**: Comprehensive documentation for new functions
3. **Code Quality**: Follow single responsibility principle
4. **Performance**: No performance regression

## Implementation Plan

### Phase 1: Analyze Current Duplication
1. **Identify Duplicate Patterns**: Document all duplicate parsing logic
2. **Map Dependencies**: Understand how each module uses parsing
3. **Assess Impact**: Determine which modules need changes

### Phase 2: Create Unified Parsing Functions
1. **Design Interface**: Create `parse_expressions_batch()` function
2. **Implement Logic**: Consolidate all parsing logic into shared utilities
3. **Add Error Handling**: Unified error handling and logging
4. **Add Tests**: Comprehensive unit tests for unified functions

### Phase 3: Refactor Executor Modules
1. **Update Single Executor**: Remove duplicate parsing, use unified functions
2. **Update Range Executor**: Remove duplicate parsing, use unified functions
3. **Update HashRange Executor**: Remove duplicate parsing, use unified functions
4. **Update Coordination Module**: Remove duplicate parsing, use unified functions
5. **Update Validation Module**: Remove duplicate parsing, use unified functions

### Phase 4: Testing and Validation
1. **Unit Tests**: Test unified parsing functions thoroughly
2. **Integration Tests**: Test all executor modules with unified parsing
3. **Performance Tests**: Ensure no performance regression
4. **Regression Tests**: Ensure all existing functionality works

## Verification

### Test Plan
1. **Unit Tests**: Test unified parsing functions with various inputs
2. **Integration Tests**: Test all executor modules with unified parsing
3. **Performance Tests**: Benchmark parsing performance before/after
4. **Regression Tests**: Run existing test suite to ensure no regressions

### Success Criteria
- [ ] `parse_expressions_batch()` function handles all parsing
- [ ] `collect_expressions_from_schema()` function handles all collection
- [ ] Duplicate parsing logic removed from executor modules
- [ ] All existing parsing behavior preserved
- [ ] Test coverage maintained >90%
- [ ] No regressions in existing functionality

## Files Modified

### New Files
- None (consolidating existing functionality)

### Modified Files
- `src/transform/shared_utilities.rs` - Enhanced with unified parsing functions
- `src/transform/single_executor.rs` - Remove duplicate parsing, use unified functions
- `src/transform/range_executor.rs` - Remove duplicate parsing, use unified functions
- `src/transform/hash_range_executor.rs` - Remove duplicate parsing, use unified functions
- `src/transform/coordination.rs` - Remove duplicate parsing, use unified functions
- `src/transform/validation.rs` - Remove duplicate parsing, use unified functions

### Test Files
- `tests/unit/transform/shared_utilities_tests.rs` - Enhanced tests for unified parsing
- `tests/integration/transform_integration_tests.rs` - Integration tests for all executors
