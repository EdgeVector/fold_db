# PBI-DTS-DEDUP-1: Eliminate Duplicate Code Patterns in Declarative Transforms

[View in Backlog](../backlog.md#user-content-DTS-DEDUP-1)

## Overview

This PBI addresses significant code duplication identified in the declarative transforms feature. Analysis revealed 40-50% of the codebase contains duplicate patterns across field alignment validation, expression parsing, result aggregation, error handling, and expression collection. This duplication increases maintenance burden, creates inconsistency risks, and violates DRY principles.

## Problem Statement

The declarative transforms system has evolved with significant code duplication across multiple executor modules:

1. **Field Alignment Validation** - Identical validation logic repeated in 5+ modules
2. **Expression Parsing** - Same parsing patterns duplicated across executor modules  
3. **Result Aggregation** - Similar result processing logic in multiple places
4. **Error Handling** - Repeated error conversion and formatting patterns
5. **Expression Collection** - Identical schema expression collection logic

This duplication creates several problems:
- **Maintenance Burden**: Changes must be made in multiple places
- **Inconsistency Risk**: Different modules may diverge over time
- **Code Bloat**: 40-50% of the codebase is duplicated
- **Testing Complexity**: Same logic tested multiple times
- **Performance Impact**: Redundant code execution

## User Stories

### Primary Story
**As a developer**, I want to eliminate duplicate code patterns in declarative transforms **so that** I can reduce maintenance burden, improve consistency, and reduce the codebase by 40-50%.

### Supporting Stories
- **As a developer**, I want unified field alignment validation **so that** I don't have to maintain identical validation logic in multiple modules
- **As a developer**, I want consolidated expression parsing **so that** parsing logic is consistent across all executor modules
- **As a developer**, I want unified result aggregation **so that** result processing follows the same patterns everywhere
- **As a developer**, I want standardized error handling **so that** error messages and handling are consistent
- **As a developer**, I want consolidated expression collection **so that** schema expression extraction is unified

## Technical Approach

### Phase 1: Consolidate Field Alignment Validation
- **Target**: Eliminate duplicate validation logic in 5+ modules
- **Approach**: Enhance `src/transform/validation.rs` as the single source of truth
- **Implementation**: 
  - Create unified `validate_field_alignment_unified()` function
  - Remove duplicate validation from executor modules
  - Use dependency injection for validation in executors
  - Maintain existing validation behavior

### Phase 2: Consolidate Expression Parsing
- **Target**: Eliminate duplicate parsing logic across executor modules
- **Approach**: Expand `src/transform/shared_utilities.rs` with parsing functions
- **Implementation**:
  - Create `parse_expressions_batch()` function
  - Create `collect_expressions_from_schema()` function
  - Remove duplicate parsing from executor modules
  - Maintain existing parsing behavior

### Phase 3: Consolidate Result Aggregation
- **Target**: Unify result processing patterns across modules
- **Approach**: Enhance `src/transform/aggregation.rs` with unified functions
- **Implementation**:
  - Create `aggregate_results_unified()` function
  - Create `process_execution_results()` function
  - Remove duplicate aggregation from executor modules
  - Maintain existing result formats

### Phase 4: Standardize Error Handling
- **Target**: Unify error handling patterns across modules
- **Approach**: Expand error handling utilities
- **Implementation**:
  - Create `format_validation_errors()` function
  - Create `format_parsing_errors()` function
  - Standardize error message formats
  - Maintain existing error behavior

### Phase 5: Create Base Executor Trait
- **Target**: Define common executor interface
- **Approach**: Create trait with shared behavior
- **Implementation**:
  - Define `DeclarativeExecutor` trait
  - Implement common functionality in trait
  - Refactor executors to use trait
  - Maintain existing executor behavior

## UX/UI Considerations

This PBI is focused on backend code deduplication and does not affect user-facing functionality. The consolidation will be transparent to users while improving system maintainability and consistency.

## Acceptance Criteria

### Primary Acceptance Criteria
1. **Code Duplication Reduction**: Achieve 40-50% reduction in duplicate code patterns
2. **Functionality Preservation**: All existing functionality continues to work unchanged
3. **Performance Maintenance**: No performance degradation from consolidation
4. **Test Coverage**: Comprehensive testing maintains >90% coverage

### Detailed Acceptance Criteria

#### Field Alignment Validation Consolidation
- [ ] Single `validate_field_alignment_unified()` function handles all validation
- [ ] Duplicate validation logic removed from 5+ modules
- [ ] All existing validation behavior preserved
- [ ] Validation performance maintained or improved

#### Expression Parsing Consolidation
- [ ] `parse_expressions_batch()` function handles all parsing
- [ ] `collect_expressions_from_schema()` function handles all collection
- [ ] Duplicate parsing logic removed from executor modules
- [ ] All existing parsing behavior preserved

#### Result Aggregation Consolidation
- [ ] `aggregate_results_unified()` function handles all aggregation
- [ ] `process_execution_results()` function handles all processing
- [ ] Duplicate aggregation logic removed from executor modules
- [ ] All existing result formats preserved

#### Error Handling Standardization
- [ ] `format_validation_errors()` function handles all validation errors
- [ ] `format_parsing_errors()` function handles all parsing errors
- [ ] Error message formats standardized across modules
- [ ] All existing error behavior preserved

#### Base Executor Trait Implementation
- [ ] `DeclarativeExecutor` trait defines common interface
- [ ] Common functionality implemented in trait
- [ ] All executors refactored to use trait
- [ ] All existing executor behavior preserved

### Quality Criteria
- [ ] **Code Quality**: All consolidated functions follow single responsibility principle
- [ ] **Documentation**: All new functions have comprehensive documentation
- [ ] **Testing**: All consolidated functions have unit tests
- [ ] **Performance**: No performance regression from consolidation
- [ ] **Maintainability**: Code is easier to maintain and extend

## Dependencies

### Internal Dependencies
- **DTS-REFACTOR-1**: Completed architectural refactoring provides clean foundation
- **DTS-REVIEW-1**: Completed system review identified duplication patterns
- **Existing Test Suite**: Comprehensive tests ensure functionality preservation

### External Dependencies
- **Rust Compiler**: No specific version requirements
- **Testing Framework**: Existing test infrastructure sufficient

## Open Questions

1. **Performance Impact**: Should we measure performance before/after consolidation?
2. **Migration Strategy**: Should consolidation be done incrementally or all at once?
3. **Backward Compatibility**: Do we need to maintain deprecated functions during transition?
4. **Documentation**: Should we create migration guide for developers?

## Related Tasks

This PBI will be broken down into specific tasks covering each phase of consolidation. Tasks will be created as part of the implementation planning process.

## Success Metrics

- **Code Reduction**: 40-50% reduction in duplicate code patterns
- **Maintainability**: Reduced time to implement changes across modules
- **Consistency**: Unified behavior across all executor modules
- **Quality**: Improved code quality metrics (cyclomatic complexity, etc.)
- **Performance**: No performance regression from consolidation

## Risk Assessment

### Low Risk
- **Functionality Preservation**: Existing tests ensure behavior preservation
- **Performance Impact**: Consolidation should improve performance
- **Code Quality**: Consolidation improves code quality

### Medium Risk
- **Integration Issues**: Multiple modules affected simultaneously
- **Testing Complexity**: Need to ensure all paths still work
- **Developer Learning**: Developers need to learn new consolidated APIs

### Mitigation Strategies
- **Incremental Implementation**: Implement consolidation phase by phase
- **Comprehensive Testing**: Maintain >90% test coverage throughout
- **Documentation**: Provide clear migration documentation
- **Code Reviews**: Thorough review of all consolidation changes
