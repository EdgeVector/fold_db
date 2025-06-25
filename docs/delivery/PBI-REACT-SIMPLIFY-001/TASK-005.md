# TASK-005: Constants Extraction and Configuration Centralization

[Back to task list](./tasks.md)

## Description

Extract all magic numbers, hardcoded strings, and configuration values into centralized constant files to improve maintainability, reduce errors, and ensure DRY compliance throughout the React application. This task addresses the scattered configuration values found in components like the hardcoded tab IDs in [`App.jsx:21`](../../../src/datafold_node/static-react/src/App.jsx:21), API endpoints in various files, and styling constants throughout the codebase.

The extraction will create a hierarchical configuration system that groups related constants and makes them easily discoverable and maintainable, following the project's Section 2.1.12 requirement for named constants.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 17:30:00 | Created | N/A | Proposed | Task file created for constants extraction | System |

## Requirements

### Core Requirements
- Extract all magic numbers and hardcoded strings into named constants
- Create hierarchical configuration structure grouped by domain
- Ensure type safety with TypeScript constant definitions
- Maintain backward compatibility during migration
- Follow Section 2.1.12 requirements for named constant usage

### Required Constants (Section 2.1.12)
All constants identified and extracted during this task must follow the naming pattern and usage requirements outlined in Section 2.1.12 of the .cursorrules policy.

### DRY Compliance Requirements
- Single source of truth for all configuration values
- Elimination of duplicate constant definitions across files
- Centralized management of related configuration groups
- Shared configuration for common UI patterns and behaviors

### SCHEMA-002 Compliance
- Schema state constants must align with backend state definitions
- Access control constants must enforce approved-only operations
- Error message constants must clearly indicate schema state violations

## Implementation Plan

### Phase 1: Identify and Catalog Constants
1. **Audit Existing Codebase**
   - Scan all React components for magic numbers and hardcoded strings
   - Identify configuration values in API clients and utilities
   - Document styling constants and UI configuration
   - Catalog business logic constants

2. **Categorize Constants**
   ```typescript
   // UI Constants
   const UI_CONSTANTS = {
     TRANSITIONS: {
       DEFAULT_DURATION_MS: 200,
       SLOW_DURATION_MS: 500,
       FAST_DURATION_MS: 100
     },
     BREAKPOINTS: {
       MOBILE_MAX_WIDTH: 768,
       TABLET_MAX_WIDTH: 1024,
       DESKTOP_MIN_WIDTH: 1025
     },
     Z_INDEX: {
       DROPDOWN: 10,
       MODAL: 50,
       TOOLTIP: 100,
       OVERLAY: 1000
     }
   };
   
   // API Constants
   const API_CONSTANTS = {
     ENDPOINTS: {
       SCHEMAS: '/api/schemas',
       MUTATIONS: '/api/mutations',
       QUERIES: '/api/queries',
       SECURITY: '/api/security'
     },
     TIMEOUTS: {
       DEFAULT_MS: 30000,
       UPLOAD_MS: 120000,
       WEBSOCKET_MS: 5000
     },
     RETRY: {
       MAX_ATTEMPTS: 3,
       DELAY_MS: 1000,
       BACKOFF_MULTIPLIER: 2
     }
   };
   ```

### Phase 2: Create Configuration Structure
1. **Domain-Specific Constants**
   ```typescript
   // Schema Domain Constants
   export const SCHEMA_CONSTANTS = {
     STATES: {
       AVAILABLE: 'available' as const,
       APPROVED: 'approved' as const,
       BLOCKED: 'blocked' as const
     },
     FIELD_TYPES: {
       STRING: 'string' as const,
       NUMBER: 'number' as const,
       BOOLEAN: 'boolean' as const,
       OBJECT: 'object' as const
     },
     VALIDATION: {
       NAME_MAX_LENGTH: 255,
       FIELD_NAME_REGEX: /^[a-zA-Z][a-zA-Z0-9_]*$/,
       SCHEMA_SIZE_LIMIT_BYTES: 1048576 // 1MB
     }
   };
   
   // Form Constants
   export const FORM_CONSTANTS = {
     VALIDATION: {
       DEBOUNCE_MS: 300,
       MIN_PASSWORD_LENGTH: 8,
       MAX_TEXT_LENGTH: 1000
     },
     FIELD_SIZES: {
       SMALL: 'sm' as const,
       MEDIUM: 'md' as const,
       LARGE: 'lg' as const
     }
   };
   ```

2. **Application Configuration**
   ```typescript
   export const APP_CONFIG = {
     DEFAULT_TAB: 'keys',
     AUTHENTICATION: {
       SESSION_TIMEOUT_MS: 3600000, // 1 hour
       KEY_REFRESH_INTERVAL_MS: 300000 // 5 minutes
     },
     CACHE: {
       DEFAULT_TTL_MS: 300000, // 5 minutes
       MAX_ENTRIES: 1000,
       CLEANUP_INTERVAL_MS: 60000 // 1 minute
     },
     LOGGING: {
       MAX_LOG_ENTRIES: 500,
       LOG_LEVELS: ['error', 'warn', 'info', 'debug'] as const
     }
   };
   ```

### Phase 3: Extract Component Constants
1. **Tab Navigation Constants**
   - Extract tab IDs and labels from [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx)
   - Define tab configuration structure
   - Include authentication requirements per tab

2. **Form Field Constants**
   - Extract field validation rules from form components
   - Define common input sizes and styling
   - Include error message templates

3. **Schema Operation Constants**
   - Extract state transition rules (SCHEMA-001 compliance)
   - Define operation timeout values
   - Include error message constants

### Phase 4: Create Styling Constants
1. **Color Palette**
   ```typescript
   export const COLORS = {
     PRIMARY: {
       50: '#eff6ff',
       100: '#dbeafe',
       500: '#3b82f6',
       600: '#2563eb',
       900: '#1e3a8a'
     },
     STATUS: {
       SUCCESS: '#10b981',
       WARNING: '#f59e0b',
       ERROR: '#ef4444',
       INFO: '#3b82f6'
     },
     SCHEMA_STATES: {
       APPROVED: 'bg-green-100 text-green-800',
       AVAILABLE: 'bg-blue-100 text-blue-800',
       BLOCKED: 'bg-red-100 text-red-800'
     }
   };
   ```

2. **Layout Constants**
   ```typescript
   export const LAYOUT = {
     SIDEBAR_WIDTH: 320,
     HEADER_HEIGHT: 64,
     FOOTER_HEIGHT: 48,
     CONTENT_PADDING: 24,
     BORDER_RADIUS: {
       SMALL: 4,
       MEDIUM: 8,
       LARGE: 12
     }
   };
   ```

### Phase 5: Migration and Integration
1. **Replace Magic Numbers**
   - Systematically replace hardcoded values with named constants
   - Update import statements across all files
   - Verify functionality remains unchanged

2. **Update Component Usage**
   - Refactor components to use centralized constants
   - Remove duplicate constant definitions
   - Add TypeScript type safety

3. **Configuration Validation**
   - Add runtime validation for critical configuration values
   - Include development-time warnings for invalid configurations
   - Implement configuration loading and validation

## Verification

### Unit Testing Requirements
- [ ] All constants properly exported and importable
- [ ] TypeScript compilation succeeds with strict type checking
- [ ] No duplicate constant definitions exist across codebase
- [ ] Configuration validation works for all constant groups
- [ ] Magic number detection tools report zero violations

### Integration Testing Requirements
- [ ] All components function correctly with extracted constants
- [ ] UI styling maintained with new color and layout constants
- [ ] API operations work correctly with extracted endpoint constants
- [ ] Form validation behaves consistently with extracted validation constants

### Maintainability Requirements
- [ ] Constants are logically grouped and easily discoverable
- [ ] Configuration changes can be made in single locations
- [ ] TypeScript provides proper autocompletion for all constants
- [ ] Documentation clearly explains constant usage patterns

### Documentation Requirements
- [ ] All constant groups documented with usage examples
- [ ] Migration guide created for updating component imports
- [ ] Configuration guide created for customizing application behavior
- [ ] Type definitions documented for all constant structures

## Files Modified

### Created Files
- `src/datafold_node/static-react/src/constants/index.ts`
- `src/datafold_node/static-react/src/constants/app.ts`
- `src/datafold_node/static-react/src/constants/api.ts`
- `src/datafold_node/static-react/src/constants/ui.ts`
- `src/datafold_node/static-react/src/constants/schema.ts`
- `src/datafold_node/static-react/src/constants/form.ts`
- `src/datafold_node/static-react/src/constants/colors.ts`
- `src/datafold_node/static-react/src/constants/layout.ts`
- `src/datafold_node/static-react/src/types/constants.ts`

### Modified Files
- `src/datafold_node/static-react/src/App.jsx` - Use tab and authentication constants
- `src/datafold_node/static-react/src/components/tabs/*.jsx` - Use domain-specific constants
- `src/datafold_node/static-react/src/api/*.ts` - Use API endpoint and timeout constants
- `src/datafold_node/static-react/src/utils/*.ts` - Use validation and utility constants
- `src/datafold_node/static-react/src/store/*.ts` - Use Redux and state constants
- `src/datafold_node/static-react/src/styles/*.css` - Use color and layout constants

### Test Files
- `src/datafold_node/static-react/src/constants/__tests__/constants.test.ts`
- `src/datafold_node/static-react/src/constants/__tests__/validation.test.ts`
- `src/datafold_node/static-react/src/test/utils/constantsHelpers.ts`

## Rollback Plan

If issues arise during constants extraction:

1. **Incremental Rollback**: Revert one constant group at a time to identify issues
2. **Import Restoration**: Restore hardcoded values in components if import issues occur
3. **Build Verification**: Ensure TypeScript compilation succeeds after each rollback step
4. **Functionality Testing**: Verify all features work correctly with rollback changes
5. **Performance Monitoring**: Ensure no performance degradation from constant extraction