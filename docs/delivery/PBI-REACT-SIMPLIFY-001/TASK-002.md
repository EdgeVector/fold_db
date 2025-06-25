# TASK-002: Component Extraction and Modularization

[Back to task list](./tasks.md)

## Description

Extract reusable UI components from monolithic tab components to improve code organization, testability, and reusability. This task focuses on extracting the tab navigation system from [`App.jsx`](../../../src/datafold_node/static-react/src/App.jsx), reusable form field components, and schema operation components.

The primary targets are the hardcoded tab navigation in [`App.jsx:181-277`](../../../src/datafold_node/static-react/src/App.jsx:181), form field patterns repeated across mutation and query tabs, and schema approval/blocking operations in [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx).

## Status History

| Timestamp | Event Type | From Status | To Status | Details | User |
|-----------|------------|-------------|-----------|---------|------|
| 2025-06-24 17:30:00 | Created | N/A | Proposed | Task file created for component extraction | System |

## Requirements

### Core Requirements
- Extract `<TabNavigation>` component from App.jsx
- Create reusable form field components (`<TextField>`, `<SelectField>`, `<SchemaSelector>`)
- Extract schema operation components (`<SchemaActions>`, `<SchemaStatusBadge>`)
- Maintain existing accessibility features and styling
- Ensure TypeScript compliance and proper prop interfaces

### Required Constants (Section 2.1.12)
```typescript
const TAB_TRANSITION_DURATION_MS = 200;
const FORM_FIELD_DEBOUNCE_MS = 300;
const SCHEMA_BADGE_COLORS = {
  approved: 'bg-green-100 text-green-800',
  available: 'bg-blue-100 text-blue-800',
  blocked: 'bg-red-100 text-red-800'
};
const COMPONENT_Z_INDEX = {
  dropdown: 10,
  modal: 50,
  tooltip: 100
};
```

### DRY Compliance Requirements
- Single definition of tab navigation logic
- Reusable form field validation and styling
- Centralized schema state visualization
- Shared button and input styling patterns
- Common loading and error state handling

### SCHEMA-002 Compliance
- Tab navigation must respect authentication state
- Schema operations must validate approval state before execution
- Form components must enforce schema access restrictions
- Error messages must indicate when operations are blocked due to schema state

## Implementation Plan

### Phase 1: Extract TabNavigation Component
1. **Extract Tab Logic**
   - Move tab definition and navigation from [`App.jsx:181-277`](../../../src/datafold_node/static-react/src/App.jsx:181)
   - Create configurable tab system with authentication awareness
   - Implement tab state management with proper transitions

2. **Tab Configuration**
   ```typescript
   interface TabConfig {
     id: string;
     label: string;
     requiresAuth: boolean;
     icon?: string;
     disabled?: boolean;
   }
   
   const DEFAULT_TABS: TabConfig[] = [
     { id: 'schemas', label: 'Schemas', requiresAuth: true },
     { id: 'query', label: 'Query', requiresAuth: true },
     { id: 'mutation', label: 'Mutation', requiresAuth: true },
     { id: 'keys', label: 'Keys', requiresAuth: false }
   ];
   ```

3. **TabNavigation Interface**
   ```typescript
   interface TabNavigationProps {
     tabs: TabConfig[];
     activeTab: string;
     isAuthenticated: boolean;
     onTabChange: (tabId: string) => void;
     className?: string;
   }
   ```

### Phase 2: Create Reusable Form Components
1. **TextField Component**
   - Extract common text input patterns from mutation and query forms
   - Add validation state visualization
   - Include debounced onChange with `FORM_FIELD_DEBOUNCE_MS`
   - Support required field indication

2. **SelectField Component**
   - Extract schema selector patterns from [`MutationTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/MutationTab.jsx)
   - Add loading and error states
   - Support grouped options and search functionality
   - Include proper ARIA attributes for accessibility

3. **SchemaSelector Component**
   - Refactor existing [`SchemaSelector`](../../../src/datafold_node/static-react/src/components/tabs/mutation/SchemaSelector.jsx)
   - Add SCHEMA-002 compliance (filter to approved schemas only)
   - Include range schema indicators
   - Support mutation type selection

4. **Form Field Interfaces**
   ```typescript
   interface TextFieldProps {
     name: string;
     label: string;
     value: string;
     onChange: (value: string) => void;
     required?: boolean;
     disabled?: boolean;
     error?: string;
     placeholder?: string;
     type?: 'text' | 'number' | 'email';
   }
   
   interface SelectFieldProps {
     name: string;
     label: string;
     value: string;
     options: Array<{ value: string; label: string; disabled?: boolean }>;
     onChange: (value: string) => void;
     required?: boolean;
     loading?: boolean;
     error?: string;
   }
   ```

### Phase 3: Extract Schema Operation Components
1. **SchemaStatusBadge Component**
   - Extract state visualization from [`SchemaTab.jsx`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx)
   - Use `SCHEMA_BADGE_COLORS` constants for consistent styling
   - Add range schema indicators
   - Support custom styling and sizes

2. **SchemaActions Component**
   - Extract approval/blocking actions from SchemaTab
   - Implement SCHEMA-002 state transition logic
   - Add confirmation dialogs for destructive actions
   - Include loading states during operations

3. **SchemaField Component**
   - Extract field rendering from [`SchemaTab.jsx:139-209`](../../../src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx:139)
   - Add range key highlighting
   - Include permission policy visualization
   - Support field expansion and collapse

4. **Schema Component Interfaces**
   ```typescript
   interface SchemaStatusBadgeProps {
     state: 'approved' | 'available' | 'blocked';
     isRangeSchema?: boolean;
     size?: 'sm' | 'md' | 'lg';
     className?: string;
   }
   
   interface SchemaActionsProps {
     schema: Schema;
     onApprove: (name: string) => Promise<void>;
     onBlock: (name: string) => Promise<void>;
     onUnload: (name: string) => Promise<void>;
     disabled?: boolean;
   }
   ```

### Phase 4: Integration and Layout Components
1. **Create OperationLayout Component**
   - Standardize layout for tab content areas
   - Include common error and loading states
   - Add result display areas
   - Support responsive design patterns

2. **Update Existing Components**
   - Refactor all tab components to use new reusable components
   - Remove duplicated styling and logic
   - Maintain existing functionality and accessibility
   - Ensure proper error boundaries

## Verification

### Unit Testing Requirements
- [ ] `<TabNavigation>` component tested with authentication scenarios
- [ ] Form field components tested with validation states
- [ ] Schema operation components tested with all schema states
- [ ] Component accessibility tested with screen readers
- [ ] Responsive design tested across viewport sizes
- [ ] Component prop validation tested with TypeScript

### Integration Testing Requirements
- [ ] Tab navigation tested with component integration
- [ ] Form submission tested with extracted field components
- [ ] Schema operations tested end-to-end with new components
- [ ] Component reusability tested across multiple contexts
- [ ] SCHEMA-002 compliance verified in all schema components

### Visual Regression Testing
- [ ] Tab navigation appearance preserved
- [ ] Form field styling matches existing design
- [ ] Schema badges maintain visual consistency
- [ ] Button and interaction states preserved
- [ ] Loading and error states properly displayed

### Documentation Requirements
- [ ] Component prop interfaces documented with TypeScript
- [ ] Usage examples provided for each component
- [ ] Storybook stories created for visual documentation
- [ ] Migration guide created for consuming components

## Files Modified

### Created Files
- `src/datafold_node/static-react/src/components/ui/TabNavigation.tsx`
- `src/datafold_node/static-react/src/components/ui/TextField.tsx`
- `src/datafold_node/static-react/src/components/ui/SelectField.tsx`
- `src/datafold_node/static-react/src/components/ui/SchemaStatusBadge.tsx`
- `src/datafold_node/static-react/src/components/ui/SchemaActions.tsx`
- `src/datafold_node/static-react/src/components/ui/SchemaField.tsx`
- `src/datafold_node/static-react/src/components/ui/OperationLayout.tsx`
- `src/datafold_node/static-react/src/types/components.ts`

### Modified Files
- `src/datafold_node/static-react/src/App.jsx` - Use TabNavigation component
- `src/datafold_node/static-react/src/components/tabs/MutationTab.jsx` - Use form field components
- `src/datafold_node/static-react/src/components/tabs/QueryTab.jsx` - Use form field components
- `src/datafold_node/static-react/src/components/tabs/SchemaTab.jsx` - Use schema components
- `src/datafold_node/static-react/src/components/tabs/mutation/SchemaSelector.jsx` - Refactor or deprecate

### Test Files
- `src/datafold_node/static-react/src/components/ui/__tests__/TabNavigation.test.tsx`
- `src/datafold_node/static-react/src/components/ui/__tests__/TextField.test.tsx`
- `src/datafold_node/static-react/src/components/ui/__tests__/SelectField.test.tsx`
- `src/datafold_node/static-react/src/components/ui/__tests__/SchemaComponents.test.tsx`
- `src/datafold_node/static-react/src/test/integration/ComponentIntegration.test.tsx`

## Rollback Plan

If issues arise during component extraction:

1. **Component Isolation**: Disable new components and restore original implementations
2. **Incremental Rollback**: Roll back one component at a time to identify issues
3. **Styling Preservation**: Ensure all styling and visual states are maintained
4. **Functionality Verification**: Test all user interactions after rollback
5. **Accessibility Compliance**: Verify screen reader compatibility is maintained