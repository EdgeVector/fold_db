# React Application Architecture Guide (v2.0.0)

This document provides a comprehensive overview of the React application architecture after the simplification initiative (PBI-REACT-SIMPLIFY-001). The new architecture emphasizes modularity, maintainability, and SCHEMA-002 compliance through centralized state management and standardized patterns.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Component Hierarchy](#component-hierarchy)
3. [State Management](#state-management)
4. [API Client System](#api-client-system)
5. [Custom Hooks](#custom-hooks)
6. [Constants and Configuration](#constants-and-configuration)
7. [SCHEMA-002 Compliance](#schema-002-compliance)
8. [Data Flow](#data-flow)
9. [Error Handling](#error-handling)
10. [Performance Considerations](#performance-considerations)
11. [Testing Strategy](#testing-strategy)
12. [Best Practices](#best-practices)

## Architecture Overview

The React application follows a **layered architecture** with clear separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                    UI LAYER (Components)                    │
├─────────────────────────────────────────────────────────────┤
│                   BUSINESS LOGIC (Hooks)                   │
├─────────────────────────────────────────────────────────────┤
│                  STATE MANAGEMENT (Redux)                  │
├─────────────────────────────────────────────────────────────┤
│                   API CLIENT LAYER                         │
├─────────────────────────────────────────────────────────────┤
│              CONSTANTS & CONFIGURATION                     │
└─────────────────────────────────────────────────────────────┘
```

### Key Architectural Principles

1. **Single Responsibility**: Each component, hook, and module has a clear, focused purpose
2. **Dependency Inversion**: High-level modules don't depend on low-level modules
3. **Open/Closed**: Open for extension, closed for modification
4. **DRY (Don't Repeat Yourself)**: Shared logic is extracted to reusable units
5. **SCHEMA-002 Compliance**: Enforced at the architectural level

### Architecture Benefits

- **Reduced Complexity**: Eliminated prop drilling and state duplication
- **Better Testability**: Isolated business logic in testable units
- **Improved Performance**: Optimized re-renders and request deduplication
- **Enhanced Maintainability**: Clear separation of concerns
- **SCHEMA-002 Enforcement**: Built-in compliance at multiple layers

## Component Hierarchy

The component structure follows a **hierarchical organization** with clear responsibilities:

```
src/components/
├── TabNavigation.jsx           # Main navigation component
├── form/                       # Form components
│   ├── FieldWrapper.jsx       # Form field container
│   ├── TextField.jsx          # Text input component
│   ├── SelectField.jsx        # Select dropdown component
│   ├── NumberField.jsx        # Number input component
│   └── RangeField.jsx         # Range-specific input
├── schema/                     # Schema-related components
│   ├── SchemaStatusBadge.jsx  # Schema state indicator
│   └── SchemaSelector.jsx     # Schema selection component
└── tabs/                       # Tab content components
    ├── mutation/               # Mutation tab components
    └── query/                  # Query tab components
```

### Component Design Patterns

#### 1. Container vs. Presentational Components

**Container Components** (Smart Components):
- Manage state and business logic
- Connect to Redux store
- Handle API calls via custom hooks
- Pass data down to presentational components

**Presentational Components** (Dumb Components):
- Focus on UI rendering
- Receive data via props
- Emit events via callback props
- No direct state management or API calls

#### 2. Compound Components

Form components use the compound pattern for flexibility:

```jsx
// Compound component pattern
<FieldWrapper label="Schema Name" required>
  <TextField
    value={schemaName}
    onChange={setSchemaName}
    placeholder="Enter schema name"
  />
</FieldWrapper>
```

#### 3. Render Props / Children as Functions

For advanced customization:

```jsx
<SchemaSelector>
  {({ schemas, isLoading, error }) => (
    <CustomSchemaDisplay
      schemas={schemas}
      loading={isLoading}
      error={error}
    />
  )}
</SchemaSelector>
```

## State Management

The application uses **Redux Toolkit** for centralized state management with a clear, predictable state structure.

### Store Structure

```javascript
{
  auth: {
    isAuthenticated: boolean,
    privateKey: string | null,
    systemKeyId: string | null,
    // ... other auth state
  },
  schemas: {
    schemas: { [schemaName]: Schema },
    loading: {
      fetch: boolean,
      operations: { [schemaName]: boolean }
    },
    errors: {
      fetch: string | null,
      operations: { [schemaName]: string }
    },
    lastFetched: number | null,
    cache: {
      ttl: number,
      version: string,
      lastUpdated: number | null
    },
    activeSchema: string | null
  }
}
```

### State Management Patterns

#### 1. Async Thunks for API Operations

```javascript
export const fetchSchemas = createAsyncThunk(
  'schemas/fetchSchemas',
  async (params, { getState, rejectWithValue }) => {
    // Implementation with retry logic, caching, and error handling
  }
);
```

#### 2. Selectors for Derived State

```javascript
export const selectApprovedSchemas = createSelector(
  [selectAllSchemas],
  (schemas) => Object.values(schemas).filter(
    schema => schema.state === SCHEMA_STATES.APPROVED
  )
);
```

#### 3. Optimistic Updates

For better UX, some operations use optimistic updates:

```javascript
// Update UI immediately, rollback if API call fails
dispatch(updateSchemaStatusOptimistic({ schemaName, newState }));
try {
  await dispatch(approveSchema({ schemaName }));
} catch (error) {
  dispatch(rollbackSchemaStatus({ schemaName }));
}
```

## API Client System

The unified API client provides **standardized HTTP communication** with built-in features:

### Client Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Specialized Clients                     │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────────────┐ │
│  │ SchemaClient │ │MutationClient│ │   SecurityClient     │ │
│  └──────────────┘ └──────────────┘ └──────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                     Core API Client                        │
│  • Authentication  • Caching       • Error Handling       │
│  • Retry Logic     • Timeouts      • Request Deduplication │
├─────────────────────────────────────────────────────────────┤
│                       HTTP Layer                           │
│              fetch() + AbortController                     │
└─────────────────────────────────────────────────────────────┘
```

### API Client Features

1. **Automatic Authentication**: Handles signing and key management
2. **Request Deduplication**: Prevents duplicate concurrent requests
3. **Caching**: Configurable response caching with TTL
4. **Retry Logic**: Exponential backoff for retryable errors
5. **Error Standardization**: Consistent error types and messages
6. **Timeout Management**: Configurable request timeouts
7. **Batch Operations**: Support for batch API calls

### Usage Patterns

```javascript
// Schema operations
import { schemaClient } from '../api';

const result = await schemaClient.approveSchema('user_profiles');
const schemas = await schemaClient.getApprovedSchemas();

// Mutation operations with validation
import { mutationClient } from '../api';

const mutation = {
  type: 'mutation',
  schema: 'user_profiles',
  mutation_type: 'create',
  data: { name: 'John', email: 'john@example.com' }
};

const result = await mutationClient.executeMutation(mutation);
```

## Custom Hooks

Custom hooks encapsulate **business logic and state management**, providing reusable interfaces for components.

### Hook Categories

#### 1. Data Fetching Hooks

**`useApprovedSchemas`**:
- Manages schema fetching and caching
- Enforces SCHEMA-002 compliance
- Provides schema validation utilities

**`useSchemaOperations`**:
- Handles schema state transitions
- Provides operation validation
- Manages loading and error states

#### 2. Business Logic Hooks

**`useRangeSchema`**:
- Range schema detection and validation
- Mutation/query formatting for range schemas
- Range key validation and processing

**`useFormValidation`**:
- Comprehensive form validation
- Debounced validation for better UX
- Schema-aware validation rules

#### 3. UI State Hooks

**`useTabNavigation`**:
- Tab state management
- Authentication-aware navigation
- URL synchronization (if needed)

### Hook Design Patterns

#### 1. Return Object Pattern

```javascript
const {
  data,
  isLoading,
  error,
  refetch,
  // Utility functions
  getItemById,
  isItemValid
} = useCustomHook();
```

#### 2. Options Parameter Pattern

```javascript
const result = useCustomHook({
  autoFetch: true,
  cacheTimeout: 300000,
  retryOnError: true
});
```

#### 3. Callback Dependency Pattern

```javascript
const processData = useCallback((data) => {
  // Processing logic
}, [dependency1, dependency2]);

const result = useCustomHook({ processor: processData });
```

## Constants and Configuration

Constants are **centrally managed** to ensure consistency and maintainability.

### Constants Organization

```
src/constants/
├── api.js          # API-related constants
├── schemas.js      # Schema-specific constants
├── ui.js           # UI and styling constants
├── redux.js        # Redux action types and defaults
└── index.js        # Centralized export
```

### Constant Categories

#### 1. API Configuration

```javascript
export const API_REQUEST_TIMEOUT_MS = 30000;
export const API_RETRY_ATTEMPTS = 3;
export const API_RETRY_DELAY_MS = 1000;
export const CACHE_CONFIG = {
  DEFAULT_TTL_MS: 300000,
  MAX_CACHE_SIZE: 100
};
```

#### 2. Schema Constants

```javascript
export const SCHEMA_STATES = {
  AVAILABLE: 'available',
  APPROVED: 'approved',
  BLOCKED: 'blocked'
};

export const VALIDATION_MESSAGES = {
  RANGE_KEY_REQUIRED: 'Range key is required for range schema mutations',
  SCHEMA_NOT_APPROVED: 'Only approved schemas can be used for this operation'
};
```

#### 3. UI Constants

```javascript
export const COMPONENT_STYLES = {
  tab: {
    base: 'px-4 py-2 text-sm font-medium transition-all duration-200',
    active: 'text-primary border-b-2 border-primary',
    inactive: 'text-gray-500 hover:text-gray-700'
  }
};
```

## SCHEMA-002 Compliance

**SCHEMA-002** compliance is enforced at multiple architectural layers to ensure only approved schemas are used for mutations and queries.

### Compliance Layers

#### 1. Hook Level
```javascript
// useApprovedSchemas enforces compliance
const { approvedSchemas, isSchemaApproved } = useApprovedSchemas();

// Only approved schemas are returned
const availableForMutation = approvedSchemas.filter(isSchemaApproved);
```

#### 2. Redux Store Level
```javascript
// Selectors automatically filter for approved schemas
export const selectApprovedSchemas = createSelector(
  [selectAllSchemas],
  (schemas) => Object.values(schemas).filter(
    schema => schema.state === SCHEMA_STATES.APPROVED
  )
);
```

#### 3. API Client Level
```javascript
// API clients validate schema state before operations
const validateSchemaForOperation = (schemaName, operation) => {
  const schema = getSchema(schemaName);
  if (!isOperationAllowed(operation, schema.state)) {
    throw new SchemaStateError('Schema not approved for operation');
  }
};
```

#### 4. Component Level
```javascript
// Components receive only approved schemas
function MutationForm() {
  const { approvedSchemas } = useApprovedSchemas();
  
  return (
    <SchemaSelector
      schemas={approvedSchemas} // Only approved schemas available
      onSelect={handleSchemaSelect}
    />
  );
}
```

### Compliance Validation

The architecture includes validation at each level:

1. **Pre-operation validation**: Before API calls
2. **Runtime validation**: During component rendering
3. **State validation**: In Redux reducers
4. **UI validation**: In form components

## Data Flow

The application follows a **unidirectional data flow** with clear patterns for different types of operations.

### Data Flow Diagram

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│  Component  │───▶│ Custom Hook  │───▶│ Redux Store │
└─────────────┘    └──────────────┘    └─────────────┘
       ▲                  │                    │
       │                  ▼                    ▼
       │            ┌──────────────┐    ┌─────────────┐
       │            │ API Client   │───▶│   Backend   │
       │            └──────────────┘    └─────────────┘
       │                  ▲                    │
       │                  │                    │
       └──────────────────┴────────────────────┘
                    Response/Updates
```

### Flow Patterns

#### 1. Data Fetching Flow

```
Component → Hook → Redux Thunk → API Client → Backend
         ← Hook ← Redux Store  ← API Client ← Backend
```

#### 2. User Action Flow

```
User Input → Component → Hook → Redux Action → Store Update
                               ↓
                         API Client → Backend
```

#### 3. Error Flow

```
Backend Error → API Client → Redux Error → Hook → Component → User
```

### State Synchronization

The architecture maintains state synchronization through:

1. **Redux as Single Source of Truth**: All shared state lives in Redux
2. **Automatic Cache Invalidation**: Smart cache invalidation strategies
3. **Optimistic Updates**: Immediate UI updates with rollback capability
4. **Real-time Updates**: WebSocket integration for real-time data (future)

## Error Handling

The architecture provides **comprehensive error handling** at every layer with user-friendly error messages.

### Error Hierarchy

```
ApiError (Base)
├── NetworkError
├── TimeoutError
├── AuthenticationError
├── SchemaStateError
├── ValidationError
└── RateLimitError
```

### Error Handling Patterns

#### 1. API Client Level

```javascript
try {
  const result = await apiClient.get('/api/schemas');
  return result;
} catch (error) {
  if (error instanceof NetworkError) {
    // Handle network issues
  } else if (error instanceof AuthenticationError) {
    // Handle auth issues
  }
  throw error; // Re-throw for higher-level handling
}
```

#### 2. Redux Level

```javascript
// Error handling in async thunks
const fetchSchemas = createAsyncThunk(
  'schemas/fetchSchemas',
  async (params, { rejectWithValue }) => {
    try {
      const schemas = await apiClient.getSchemas();
      return schemas;
    } catch (error) {
      return rejectWithValue(error.toUserMessage());
    }
  }
);
```

#### 3. Hook Level

```javascript
const useApprovedSchemas = () => {
  const error = useAppSelector(selectFetchError);
  
  return {
    // ... other returns
    error: error ? formatErrorForUser(error) : null
  };
};
```

#### 4. Component Level

```javascript
function SchemaList() {
  const { schemas, isLoading, error } = useApprovedSchemas();
  
  if (error) {
    return <ErrorBoundary error={error} />;
  }
  
  // ... rest of component
}
```

## Performance Considerations

The architecture includes several **performance optimizations**:

### 1. Redux Optimizations

- **Normalized State**: Schemas stored by ID for O(1) lookups
- **Memoized Selectors**: Using `createSelector` for expensive computations
- **Minimal Re-renders**: Precise state subscriptions

### 2. API Client Optimizations

- **Request Deduplication**: Prevents duplicate concurrent requests
- **Response Caching**: Configurable caching with TTL
- **Batch Operations**: Multiple operations in single request

### 3. Component Optimizations

- **React.memo**: Prevents unnecessary re-renders
- **useCallback/useMemo**: Memoizes functions and expensive computations
- **Code Splitting**: Lazy loading of components

### 4. Bundle Optimizations

- **Tree Shaking**: Eliminates unused code
- **Module Federation**: Shared dependencies optimization
- **Dynamic Imports**: On-demand loading

### Performance Monitoring

```javascript
// Built-in performance monitoring
const metrics = apiClient.getMetrics();
console.log('Average response time:', metrics.averageResponseTime);
console.log('Cache hit rate:', metrics.cacheHitRate);
```

## Testing Strategy

The architecture supports comprehensive testing at all levels:

### Testing Pyramid

```
┌─────────────────────────────────┐
│        E2E Tests (Few)          │  ← Full user workflows
├─────────────────────────────────┤
│     Integration Tests (Some)    │  ← API + Component integration
├─────────────────────────────────┤
│      Unit Tests (Many)          │  ← Hooks, utilities, components
└─────────────────────────────────┘
```

### Testing Patterns

#### 1. Hook Testing

```javascript
import { renderHook, act } from '@testing-library/react';
import { useApprovedSchemas } from '../useApprovedSchemas';

test('should fetch approved schemas', async () => {
  const { result } = renderHook(() => useApprovedSchemas());
  
  expect(result.current.isLoading).toBe(true);
  
  await act(async () => {
    await waitForNextUpdate();
  });
  
  expect(result.current.approvedSchemas).toHaveLength(2);
});
```

#### 2. Component Testing

```javascript
import { render, screen, fireEvent } from '@testing-library/react';
import { Provider } from 'react-redux';
import TabNavigation from '../TabNavigation';

test('should render tabs and handle clicks', () => {
  render(
    <Provider store={mockStore}>
      <TabNavigation
        activeTab="schemas"
        isAuthenticated={true}
        onTabChange={mockOnTabChange}
      />
    </Provider>
  );
  
  fireEvent.click(screen.getByText('Query'));
  expect(mockOnTabChange).toHaveBeenCalledWith('query');
});
```

#### 3. API Client Testing

```javascript
import { rest } from 'msw';
import { setupServer } from 'msw/node';
import { schemaClient } from '../api';

const server = setupServer(
  rest.get('/api/schemas', (req, res, ctx) => {
    return res(ctx.json({ data: mockSchemas }));
  })
);

test('should fetch schemas with retry', async () => {
  const schemas = await schemaClient.getSchemas();
  expect(schemas).toEqual(mockSchemas);
});
```

## Best Practices

### 1. Component Best Practices

- **Single Responsibility**: Each component has one clear purpose
- **Props Interface**: Well-defined TypeScript interfaces
- **Accessibility**: ARIA attributes and keyboard navigation
- **Error Boundaries**: Graceful error handling

### 2. Hook Best Practices

- **Pure Functions**: No side effects in custom hooks
- **Dependency Arrays**: Proper dependency management
- **Error Handling**: Comprehensive error scenarios
- **Testing**: Isolated unit tests

### 3. State Management Best Practices

- **Normalized State**: Flat, normalized data structures
- **Immutability**: Never mutate state directly
- **Minimal State**: Keep only necessary data in store
- **Clear Actions**: Descriptive action names and types

### 4. API Client Best Practices

- **Error Types**: Use specific error types
- **Timeouts**: Always set request timeouts
- **Validation**: Validate requests and responses
- **Logging**: Comprehensive request/response logging

### 5. Performance Best Practices

- **Memoization**: Memo expensive computations
- **Lazy Loading**: Load components on demand
- **Bundle Analysis**: Monitor bundle size
- **Caching**: Strategic response caching

## Migration from Legacy Architecture

For teams migrating from the previous architecture, see the comprehensive [Migration Guide](./migration.md) which includes:

- Step-by-step migration instructions
- Before/after code examples
- Common migration issues and solutions
- Testing strategies for migrated code

## Future Considerations

The architecture is designed to support future enhancements:

1. **Micro-frontends**: Modular architecture supports federation
2. **Real-time Updates**: WebSocket integration planned
3. **Offline Support**: Service worker integration possible
4. **Advanced Caching**: Redis integration for complex caching needs

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2025-06-24 | Complete architecture redesign with React simplification |
| 1.x | Prior | Legacy architecture (deprecated) |

For implementation details, see the [API Reference](../../reference/api-reference.md) and [Testing Guide](./testing.md).