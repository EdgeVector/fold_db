# PBI-REACT-SIMPLIFY-001: React Frontend Simplification and Architecture Improvement

## Overview

This Product Backlog Item addresses the increasing complexity and maintainability challenges in the React frontend codebase, implementing architectural improvements to enhance code organization, reusability, and development velocity.

## Problem Statement

The current React frontend in [`src/datafold_node/static-react/`](../../../src/datafold_node/static-react/) has evolved into a complex, tightly-coupled architecture with several critical issues:

### Identified Complexity Issues

1. **Monolithic Components**
   - [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx) contains 301 lines with mixed concerns (authentication, schema fetching, tab navigation)
   - [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx) exceeds 500 lines with business logic mixed with UI rendering
   - [`MutationTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/MutationTab.jsx) contains complex form validation and range schema logic

2. **Duplicated Logic**
   - Schema fetching logic duplicated across multiple components
   - Form validation patterns repeated in different tabs
   - Authentication state checks scattered throughout the codebase

3. **Inconsistent API Patterns**
   - Multiple API clients ([`schemaClient.ts`](../../../src/datafold_node/static-react/src/api/schemaClient.ts), [`mutationClient.ts`](../../../src/datafold_node/static-react/src/api/mutationClient.ts)) with different interfaces
   - Mixed patterns for error handling and response processing
   - Inconsistent authentication wrapper usage

4. **Limited Reusability**
   - Tab navigation hardcoded in main component
   - Form field components not extracted for reuse
   - Schema approval/blocking logic not modularized

5. **Testing Challenges**
   - Large components difficult to unit test
   - Business logic tightly coupled with UI rendering
   - Mock setup complexity due to component interdependencies

## User Stories

### As a Developer
- **US-001**: I want to have reusable custom hooks for common operations so that I can reduce code duplication and improve consistency across components
- **US-002**: I want extracted, focused components so that I can easily test and maintain individual pieces of functionality
- **US-003**: I want a unified API client interface so that I can interact with the backend consistently across all features
- **US-004**: I want centralized state management for schemas so that I can avoid prop drilling and ensure data consistency

### As a Product Owner
- **US-005**: I want improved development velocity so that new features can be implemented more quickly and with fewer bugs
- **US-006**: I want maintainable code so that technical debt doesn't impede future feature development

### As a QA Engineer
- **US-007**: I want smaller, focused components so that I can write targeted unit tests and identify issues more easily

## Technical Approach

### Architecture Principles
1. **Single Responsibility**: Each component/hook handles one primary concern
2. **Reusability**: Extract common patterns into reusable components and hooks
3. **Consistency**: Standardize API interactions and state management patterns
4. **Testability**: Design components for easy unit and integration testing

### Implementation Strategy
1. **Incremental Refactoring**: Implement changes in small, safe iterations
2. **Backward Compatibility**: Maintain existing functionality throughout the transition
3. **Test-Driven Improvements**: Add tests before and after each refactoring step
4. **SCHEMA-002 Compliance**: Ensure all schema access adheres to approval state requirements

## UX/UI Considerations

- **No Visual Changes**: This is purely an architectural improvement with no user-facing changes
- **Performance Maintenance**: Ensure refactoring doesn't negatively impact load times or responsiveness
- **Accessibility Preservation**: Maintain existing accessibility features throughout refactoring

## Acceptance Criteria

### AC-001: Custom Hooks Extraction
- [ ] `useApprovedSchemas()` hook extracts schema fetching and filtering logic
- [ ] `useRangeSchema()` hook handles range schema-specific operations
- [ ] `useFormValidation()` hook provides reusable form validation patterns
- [ ] All hooks include proper TypeScript typing and error handling

### AC-002: Component Modularity
- [ ] Tab navigation extracted into reusable `<TabNavigation>` component
- [ ] Form field components extracted for reuse across tabs
- [ ] Schema operation components modularized and tested
- [ ] All extracted components maintain existing functionality

### AC-003: State Management Consolidation
- [ ] Redux schema slice handles all schema state operations
- [ ] Schema state synchronized across all components
- [ ] Removed redundant local state management
- [ ] SCHEMA-002 compliance enforced at the store level

### AC-004: API Standardization
- [ ] Unified API client with consistent error handling
- [ ] Standardized authentication wrapper usage
- [ ] Consistent response typing across all endpoints
- [ ] Reduced API client complexity and duplication

### AC-005: Constants and Configuration
- [ ] All magic numbers extracted to named constants
- [ ] API endpoints centralized in configuration
- [ ] Component configuration externalized
- [ ] DRY principle compliance verified

### AC-006: Testing Coverage
- [ ] Unit tests added for all extracted hooks
- [ ] Component tests updated for modular components
- [ ] Integration tests verify end-to-end functionality
- [ ] Test coverage maintains or exceeds current levels

## Dependencies

- **Internal**: Redux store structure, existing API endpoints
- **External**: No new external dependencies required
- **Compliance**: Must maintain SCHEMA-002 access control requirements

## Open Questions

- Should we introduce React Query for API state management, or stick with Redux?
- What level of TypeScript strictness should we enforce in the refactored code?
- Should we establish component library patterns for future development?

## Related Tasks

All implementation tasks are detailed in [`tasks.md`](./tasks.md) with specific scope, requirements, and verification criteria.

[Back to task list](./tasks.md)