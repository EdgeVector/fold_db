# TASK-001: Extract Custom Hooks for Schema and Form Operations

[Back to task list](./tasks.md)

## Description

Extract common logic into reusable React custom hooks to reduce code duplication and improve maintainability. This task focuses on creating three primary hooks that encapsulate schema operations, range schema handling, and form validation patterns currently scattered across multiple components.

The extraction will target the schema fetching logic in [`App.jsx:42-94`](../../../src/datafold_node/static-react/src/App.jsx:42), range schema utilities used in [`MutationTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/MutationTab.jsx), and form validation patterns used across tab components.

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 17:30:00 | Created | N/A | Proposed | Task file created for custom hooks extraction | System |

## Requirements

### Core Requirements
- Extract three custom hooks: `useApprovedSchemas`, `useRangeSchema`, and `useFormValidation`
- Maintain backward compatibility with existing component interfaces
- Ensure SCHEMA-002 compliance (only approved schemas accessible for operations)
- Follow React hooks best practices and conventions

### Required Constants (Section 2.1.12)
```typescript
const SCHEMA_FETCH_RETRY_COUNT = 3;
const SCHEMA_CACHE_DURATION_MS = 300000; // 5 minutes
const FORM_VALIDATION_DEBOUNCE_MS = 500;
const RANGE_SCHEMA_FIELD_PREFIX = 'range_';
```

### DRY Compliance Requirements
- Single source of truth for schema fetching logic (currently duplicated in App.jsx and SchemaTab.jsx)
- Centralized range schema validation (currently in MutationTab.jsx and rangeSchemaUtils.js)
- Unified form validation patterns across all tab components
- Shared error handling and loading states

### SCHEMA-002 Compliance
- `useApprovedSchemas` must filter and return only schemas with `state: 'approved'`
- Hook must enforce access control at the data layer
- Validation functions must check schema state before operations
- Error messages must clearly indicate when operations are blocked due to schema state

## Implementation Plan

### Phase 1: Create useApprovedSchemas Hook
1. **Extract Schema Fetching Logic**
   - Move [`App.jsx:42-94`](../../../src/datafold_node/static-react/src/App.jsx:42) schema fetching to custom hook
   - Implement caching with `SCHEMA_CACHE_DURATION_MS` constant
   - Add retry mechanism with `SCHEMA_FETCH_RETRY_COUNT` constant
   - Return loading, error, and schemas state

2. **SCHEMA-002 Enforcement**
   - Filter schemas to return only those with `state: 'approved'`
   - Provide separate methods for administrative schema listing (available, blocked)
   - Include validation functions that check schema state

3. **Hook Interface Design**
   ```typescript
   interface UseApprovedSchemasResult {
     approvedSchemas: Schema[];
     isLoading: boolean;
     error: string | null;
     refetch: () => Promise<void>;
     getSchemaByName: (name: string) => Schema | null;
     isSchemaApproved: (name: string) => boolean;
   }
   ```

### Phase 2: Create useRangeSchema Hook
1. **Extract Range Schema Logic**
   - Move range schema utilities from [`rangeSchemaUtils.js`](../../../src/datafold_node/static-react/src/utils/rangeSchemaUtils.js)
   - Integrate with form handling for mutation operations
   - Add range field validation with debounced input

2. **Range Schema Operations**
   - Implement `formatRangeMutation` with proper key handling
   - Add `validateRangeKey` with `FORM_VALIDATION_DEBOUNCE_MS` debouncing
   - Provide `getRangeFields` and `getNonRangeFields` utilities

3. **Hook Interface Design**
   ```typescript
   interface UseRangeSchemaResult {
     isRangeSchema: (schema: Schema) => boolean;
     formatRangeMutation: (schema: Schema, type: string, rangeKey: string, data: any) => any;
     validateRangeKey: (value: string, required: boolean) => string | null;
     getRangeFields: (schema: Schema) => string[];
     getNonRangeFields: (schema: Schema) => Record<string, any>;
   }
   ```

### Phase 3: Create useFormValidation Hook
1. **Extract Validation Patterns**
   - Identify common validation patterns from MutationTab and QueryTab
   - Implement debounced validation with `FORM_VALIDATION_DEBOUNCE_MS`
   - Add field-level and form-level validation support

2. **Validation Features**
   - Required field validation
   - Type-specific validation (string, number, boolean)
   - Custom validation rule support
   - Error message formatting

3. **Hook Interface Design**
   ```typescript
   interface UseFormValidationResult {
     validate: (fieldName: string, value: any, rules: ValidationRule[]) => string | null;
     validateForm: (data: Record<string, any>, schema: Schema) => Record<string, string>;
     isFormValid: (errors: Record<string, string>) => boolean;
     getFieldError: (fieldName: string) => string | null;
   }
   ```

### Phase 4: Integration and Testing
1. **Update Components**
   - Refactor [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx) to use `useApprovedSchemas`
   - Update [`MutationTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/MutationTab.jsx) to use new hooks
   - Modify other tab components to use appropriate hooks

2. **Maintain Functionality**
   - Ensure all existing features continue to work
   - Preserve error handling and loading states
   - Maintain SCHEMA-002 compliance throughout

## Verification

### Unit Testing Requirements
- [ ] `useApprovedSchemas` hook tested with mock API responses
- [ ] SCHEMA-002 compliance verified (only approved schemas returned)
- [ ] `useRangeSchema` hook tested with range and non-range schemas
- [ ] `useFormValidation` hook tested with various validation scenarios
- [ ] Error handling tested for network failures and invalid data
- [ ] Caching behavior verified with time-based tests

### Integration Testing Requirements
- [ ] Component integration tested with extracted hooks
- [ ] Schema state transitions tested (available → approved)
- [ ] Form submission flow tested with validation hooks
- [ ] Range schema mutations tested end-to-end
- [ ] Error states properly propagated to UI components

### Performance Requirements
- [ ] Schema caching reduces API calls by at least 70%
- [ ] Debounced validation prevents excessive validation calls
- [ ] Hook re-renders minimized through proper dependency management
- [ ] Memory usage does not increase significantly

### Documentation Requirements
- [ ] JSDoc comments added to all hook interfaces
- [ ] Usage examples provided for each hook
- [ ] Migration guide created for component updates
- [ ] TypeScript types fully documented

## Files Modified

### Created Files
- `src/datafold_node/static-react/src/hooks/useApprovedSchemas.ts`
- `src/datafold_node/static-react/src/hooks/useRangeSchema.ts`
- `src/datafold_node/static-react/src/hooks/useFormValidation.ts`
- `src/datafold_node/static-react/src/types/hooks.ts`

### Modified Files
- `src/datafold_node/static-react/src/App.jsx` - Replace schema fetching with `useApprovedSchemas`
- `src/datafold_node/static-react/src/components/tabs/MutationTab.jsx` - Integrate range schema and validation hooks
- `src/datafold_node/static-react/src/components/tabs/QueryTab.jsx` - Use validation hook
- `src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx` - Use approved schemas hook
- `src/datafold_node/static-react/src/utils/rangeSchemaUtils.js` - Deprecate in favor of hook

### Test Files
- `src/datafold_node/static-react/src/hooks/__tests__/useApprovedSchemas.test.ts`
- `src/datafold_node/static-react/src/hooks/__tests__/useRangeSchema.test.ts`
- `src/datafold_node/static-react/src/hooks/__tests__/useFormValidation.test.ts`
- `src/datafold_node/static-react/src/test/integration/HooksIntegration.test.tsx`

## Rollback Plan

If issues arise during implementation:

1. **Immediate Rollback**: Revert component changes to use original logic
2. **Hook Isolation**: Disable new hooks and restore original utility functions
3. **Incremental Rollback**: Roll back one hook at a time to identify issues
4. **Data Preservation**: Ensure no schema state or user data is lost during rollback
5. **Testing Verification**: Run full test suite after rollback to confirm stability