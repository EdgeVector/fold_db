# API-STD-1: Standardize API Client Usage Across React Codebase

## User Story
As a developer, I want all API calls in the React codebase to use the standardized API client architecture, so that the codebase is consistent, maintainable, and secure.

## Business Value
- Reduces technical debt and risk by enforcing a single, maintainable API access pattern
- Improves security by centralizing authentication and error handling
- Enables faster onboarding and easier code reviews

## Acceptance Criteria
- All direct `fetch()` calls in the React codebase are replaced with calls to the appropriate API client
- No new direct `fetch()` calls are introduced
- All constants and repeated values are defined in a single location per DRY principles
- Technical documentation is updated to reflect new API usage patterns
- All affected components and modules have passing tests

## Conditions of Satisfaction (CoS)
- [ ] 100% of identified fetch() violations are refactored (33 total violations)
- [ ] All code changes are associated with explicit, agreed-upon tasks
- [ ] Project logic documentation is updated with new/modified logic IDs
- [ ] No code outside the agreed scope is changed
- [ ] All tasks are tracked and status-synchronized per .cursorrules

## Technical Justification
Based on linting analysis, found 33 direct `fetch()` violations across 8 files that bypass shared logic, increasing risk of bugs, inconsistent error handling, and security issues. Centralizing API access enables better logging, error management, and future enhancements.

## Scope
- React frontend components in `src/datafold_node/static-react/`
- Files with direct `fetch()` usage: SchemaTab.jsx, schemaSlice.ts, StatusSection.jsx, LogSidebar.jsx, TransformsTab.jsx, IngestionTab.jsx, httpClient.ts
- Existing API clients: SchemaClient, SecurityClient, MutationClient

## Dependencies
- Existing unified API client architecture
- SCHEMA-002 compliance requirements
- Redux state management patterns

## Open Questions
- Should sample schema loading be moved to a dedicated SampleClient?
- How to handle legacy endpoints that don't follow the unified pattern?

## Related Tasks
[View task list](./tasks.md)