# Testing Guide for React Application (v2.0.0)

This guide provides comprehensive testing strategies, utilities, and best practices for the React application's simplified architecture. The testing approach follows the **testing pyramid** with emphasis on unit tests, supported by integration tests and end-to-end validation.

## Table of Contents

1. [Testing Philosophy](#testing-philosophy)
2. [Testing Pyramid](#testing-pyramid)
3. [Testing Setup](#testing-setup)
4. [Unit Testing](#unit-testing)
5. [Integration Testing](#integration-testing)
6. [Testing Utilities](#testing-utilities)
7. [Testing Patterns](#testing-patterns)
8. [Coverage Requirements](#coverage-requirements)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

## Testing Philosophy

Our testing strategy is built on these core principles:

- **Fast Feedback**: Tests should run quickly and provide immediate feedback
- **Reliability**: Tests should be deterministic and not flaky
- **Maintainability**: Tests should be easy to understand and maintain
- **Coverage**: Critical paths and business logic must be thoroughly tested
- **Isolation**: Tests should not depend on external services or state

## Testing Pyramid

```
┌─────────────────────────────────┐
│        E2E Tests (5%)           │  ← Full user workflows
├─────────────────────────────────┤
│     Integration Tests (25%)     │  ← Component + API integration
├─────────────────────────────────┤
│      Unit Tests (70%)           │  ← Hooks, utilities, components
└─────────────────────────────────┘
```

### Test Distribution

- **Unit Tests (70%)**: Individual functions, hooks, and components
- **Integration Tests (25%)**: Component interactions and API integration
- **E2E Tests (5%)**: Complete user workflows and critical paths

## Testing Setup

### Dependencies

```json
{
  "devDependencies": {
    "@testing-library/react": "^13.4.0",
    "@testing-library/jest-dom": "^5.16.5",
    "@testing-library/user-event": "^14.4.3",
    "@testing-library/react-hooks": "^8.0.1",
    "vitest": "^0.34.0",
    "jsdom": "^22.1.0",
    "msw": "^1.3.0",
    "@reduxjs/toolkit": "^1.9.0"
  }
}
```

### Test Configuration

**`vitest.config.js`**:
```javascript
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.js'],
    coverage: {
      reporter: ['text', 'html', 'clover', 'json'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.d.ts',
        '**/*.config.js'
      ],
      thresholds: {
        global: {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80
        }
      }
    }
  }
});
```

**Test Setup** (`src/test/setup.js`):
```javascript
import '@testing-library/jest-dom';
import { setupServer } from 'msw/node';
import { handlers } from './mocks/handlers';

// Setup MSW server
export const server = setupServer(...handlers);

beforeAll(() => server.listen());
afterEach(() => server.resetHandlers());
afterAll(() => server.close());

// Mock IntersectionObserver
global.IntersectionObserver = class IntersectionObserver {
  constructor() {}
  disconnect() {}
  observe() {}
  unobserve() {}
};
```

## Unit Testing

### Testing Custom Hooks

**Testing `useApprovedSchemas`**:
```javascript
import { renderHook, waitFor } from '@testing-library/react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { useApprovedSchemas } from '../useApprovedSchemas';
import { schemaSlice } from '../../store/schemaSlice';

const createTestStore = (initialState = {}) => {
  return configureStore({
    reducer: {
      schemas: schemaSlice.reducer
    },
    preloadedState: {
      schemas: {
        schemas: {},
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '1.0.0', lastUpdated: null },
        activeSchema: null,
        ...initialState
      }
    }
  });
};

const wrapper = ({ children, store }) => (
  <Provider store={store}>{children}</Provider>
);

describe('useApprovedSchemas', () => {
  test('should return approved schemas only', async () => {
    const store = createTestStore({
      schemas: {
        'schema1': { name: 'schema1', state: 'approved', fields: {} },
        'schema2': { name: 'schema2', state: 'available', fields: {} },
        'schema3': { name: 'schema3', state: 'approved', fields: {} }
      }
    });

    const { result } = renderHook(() => useApprovedSchemas(), {
      wrapper: ({ children }) => wrapper({ children, store })
    });

    expect(result.current.approvedSchemas).toHaveLength(2);
    expect(result.current.approvedSchemas.map(s => s.name)).toEqual(['schema1', 'schema3']);
  });

  test('should validate schema approval status', () => {
    const store = createTestStore({
      schemas: {
        'approved_schema': { name: 'approved_schema', state: 'approved', fields: {} },
        'available_schema': { name: 'available_schema', state: 'available', fields: {} }
      }
    });

    const { result } = renderHook(() => useApprovedSchemas(), {
      wrapper: ({ children }) => wrapper({ children, store })
    });

    expect(result.current.isSchemaApproved('approved_schema')).toBe(true);
    expect(result.current.isSchemaApproved('available_schema')).toBe(false);
    expect(result.current.isSchemaApproved('nonexistent')).toBe(false);
  });

  test('should handle refetch operation', async () => {
    const store = createTestStore();
    
    const { result } = renderHook(() => useApprovedSchemas(), {
      wrapper: ({ children }) => wrapper({ children, store })
    });

    await waitFor(() => {
      result.current.refetch();
    });

    // Verify that refetch triggers a new fetch operation
    expect(store.getState().schemas.loading.fetch).toBe(true);
  });
});
```

**Testing `useRangeSchema`**:
```javascript
import { renderHook } from '@testing-library/react';
import { useRangeSchema } from '../useRangeSchema';

describe('useRangeSchema', () => {
  const mockRangeSchema = {
    name: 'time_series',
    fields: {
      timestamp: { field_type: 'Range' },
      value: { field_type: 'Range' }
    },
    schema_type: { Range: { range_key: 'timestamp' } }
  };

  const mockStandardSchema = {
    name: 'user_profile',
    fields: {
      name: { field_type: 'String' },
      age: { field_type: 'Number' }
    }
  };

  test('should identify range schemas correctly', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.isRange(mockRangeSchema)).toBe(true);
    expect(result.current.isRange(mockStandardSchema)).toBe(false);
    expect(result.current.isRange(null)).toBe(false);
    expect(result.current.isRange({})).toBe(false);
  });

  test('should extract range key correctly', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.rangeProps.getRangeKey(mockRangeSchema)).toBe('timestamp');
    expect(result.current.rangeProps.getRangeKey(mockStandardSchema)).toBe(null);
  });

  test('should validate range key values', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.rangeProps.validateRangeKey('valid_key', true)).toBe(null);
    expect(result.current.rangeProps.validateRangeKey('', true)).toBe('Range key is required for range schema mutations');
    expect(result.current.rangeProps.validateRangeKey('   ', true)).toBe('Range key cannot be empty');
    expect(result.current.rangeProps.validateRangeKey('valid', false)).toBe(null);
  });

  test('should format range mutations correctly', () => {
    const { result } = renderHook(() => useRangeSchema());

    const mutation = result.current.rangeProps.formatRangeMutation(
      mockRangeSchema,
      'Create',
      'user123',
      { value: 42 }
    );

    expect(mutation).toEqual({
      type: 'mutation',
      schema: 'time_series',
      mutation_type: 'create',
      data: {
        timestamp: 'user123',
        value: { value: 42 }
      }
    });
  });

  test('should format range queries correctly', () => {
    const { result } = renderHook(() => useRangeSchema());

    const query = result.current.rangeProps.formatRangeQuery(
      mockRangeSchema,
      ['timestamp', 'value'],
      'user123'
    );

    expect(query).toEqual({
      type: 'query',
      schema: 'time_series',
      fields: ['timestamp', 'value'],
      range_filter: { Key: 'user123' }
    });
  });
});
```

### Testing Components

**Testing `TabNavigation`**:
```javascript
import { render, screen, fireEvent } from '@testing-library/react';
import TabNavigation from '../TabNavigation';

const mockTabs = [
  { id: 'public', label: 'Public', requiresAuth: false },
  { id: 'private', label: 'Private', requiresAuth: true }
];

describe('TabNavigation', () => {
  test('should render all tabs with correct labels', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={false}
        onTabChange={mockOnTabChange}
      />
    );

    expect(screen.getByText('Public')).toBeInTheDocument();
    expect(screen.getByText('Private')).toBeInTheDocument();
  });

  test('should disable auth-required tabs when not authenticated', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={false}
        onTabChange={mockOnTabChange}
      />
    );

    const privateTab = screen.getByRole('button', { name: /Private.*authentication required/i });
    expect(privateTab).toBeDisabled();
  });

  test('should enable all tabs when authenticated', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={true}
        onTabChange={mockOnTabChange}
      />
    );

    const publicTab = screen.getByRole('button', { name: /Public/i });
    const privateTab = screen.getByRole('button', { name: /Private/i });

    expect(publicTab).not.toBeDisabled();
    expect(privateTab).not.toBeDisabled();
  });

  test('should call onTabChange when clickable tab is clicked', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={true}
        onTabChange={mockOnTabChange}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /Private/i }));
    expect(mockOnTabChange).toHaveBeenCalledWith('private');
  });

  test('should not call onTabChange when disabled tab is clicked', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={false}
        onTabChange={mockOnTabChange}
      />
    );

    fireEvent.click(screen.getByRole('button', { name: /Private.*authentication required/i }));
    expect(mockOnTabChange).not.toHaveBeenCalled();
  });

  test('should apply correct ARIA attributes', () => {
    const mockOnTabChange = vi.fn();

    render(
      <TabNavigation
        tabs={mockTabs}
        activeTab="public"
        isAuthenticated={true}
        onTabChange={mockOnTabChange}
      />
    );

    const activeTab = screen.getByRole('button', { name: /Public/i });
    const inactiveTab = screen.getByRole('button', { name: /Private/i });

    expect(activeTab).toHaveAttribute('aria-current', 'page');
    expect(inactiveTab).not.toHaveAttribute('aria-current');
  });
});
```

### Testing Form Components

**Testing `TextField`**:
```javascript
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import TextField from '../TextField';

describe('TextField', () => {
  test('should render with label and input', () => {
    render(
      <TextField
        label="Username"
        value=""
        onChange={() => {}}
      />
    );

    expect(screen.getByLabelText('Username')).toBeInTheDocument();
    expect(screen.getByRole('textbox')).toBeInTheDocument();
  });

  test('should call onChange when value changes', async () => {
    const user = userEvent.setup();
    const mockOnChange = vi.fn();

    render(
      <TextField
        label="Username"
        value=""
        onChange={mockOnChange}
      />
    );

    const input = screen.getByRole('textbox');
    await user.type(input, 'test');

    expect(mockOnChange).toHaveBeenCalledTimes(4); // One for each character
  });

  test('should display error message when provided', () => {
    render(
      <TextField
        label="Username"
        value=""
        onChange={() => {}}
        error="Username is required"
      />
    );

    expect(screen.getByText('Username is required')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  test('should show required indicator when required', () => {
    render(
      <TextField
        label="Username"
        value=""
        onChange={() => {}}
        required
      />
    );

    expect(screen.getByText('*')).toBeInTheDocument();
  });

  test('should disable input when disabled prop is true', () => {
    render(
      <TextField
        label="Username"
        value=""
        onChange={() => {}}
        disabled
      />
    );

    expect(screen.getByRole('textbox')).toBeDisabled();
  });
});
```

## Integration Testing

### Testing Component + Hook Integration

**Testing Schema Selection Workflow**:
```javascript
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Provider } from 'react-redux';
import { rest } from 'msw';
import { server } from '../../test/setup';
import { createTestStore } from '../../test/utils/testStore';
import SchemaSelector from '../SchemaSelector';

describe('SchemaSelector Integration', () => {
  beforeEach(() => {
    server.use(
      rest.get('/api/schemas/available', (req, res, ctx) => {
        return res(ctx.json({
          data: ['user_profiles', 'time_series']
        }));
      }),
      rest.get('/api/schemas', (req, res, ctx) => {
        return res(ctx.json({
          data: {
            user_profiles: 'approved',
            time_series: 'available'
          }
        }));
      }),
      rest.get('/api/schema/user_profiles', (req, res, ctx) => {
        return res(ctx.json({
          name: 'user_profiles',
          state: 'approved',
          fields: {
            name: { field_type: 'String' },
            age: { field_type: 'Number' }
          }
        }));
      })
    );
  });

  test('should load and display approved schemas only', async () => {
    const store = createTestStore();
    const mockOnSelect = vi.fn();

    render(
      <Provider store={store}>
        <SchemaSelector onSelect={mockOnSelect} />
      </Provider>
    );

    // Wait for schemas to load
    await waitFor(() => {
      expect(screen.getByText('user_profiles')).toBeInTheDocument();
    });

    // Should not show available schemas
    expect(screen.queryByText('time_series')).not.toBeInTheDocument();
  });

  test('should handle schema selection', async () => {
    const user = userEvent.setup();
    const store = createTestStore();
    const mockOnSelect = vi.fn();

    render(
      <Provider store={store}>
        <SchemaSelector onSelect={mockOnSelect} />
      </Provider>
    );

    await waitFor(() => {
      expect(screen.getByText('user_profiles')).toBeInTheDocument();
    });

    await user.click(screen.getByText('user_profiles'));
    expect(mockOnSelect).toHaveBeenCalledWith('user_profiles');
  });

  test('should display loading state during fetch', () => {
    const store = createTestStore();

    render(
      <Provider store={store}>
        <SchemaSelector onSelect={() => {}} />
      </Provider>
    );

    expect(screen.getByText('Loading schemas...')).toBeInTheDocument();
  });

  test('should handle API errors gracefully', async () => {
    server.use(
      rest.get('/api/schemas/available', (req, res, ctx) => {
        return res(ctx.status(500), ctx.json({ error: 'Server error' }));
      })
    );

    const store = createTestStore();

    render(
      <Provider store={store}>
        <SchemaSelector onSelect={() => {}} />
      </Provider>
    );

    await waitFor(() => {
      expect(screen.getByText(/error/i)).toBeInTheDocument();
    });
  });
});
```

### Testing API Client Integration

**Testing Schema API Client**:
```javascript
import { rest } from 'msw';
import { server } from '../../test/setup';
import { createSchemaClient } from '../clients/schemaClient';
import { createMockApiClient } from '../../test/utils/apiMocks';

describe('Schema API Client Integration', () => {
  let schemaClient;

  beforeEach(() => {
    const mockBaseClient = createMockApiClient();
    schemaClient = createSchemaClient(mockBaseClient);
  });

  test('should fetch approved schemas with proper formatting', async () => {
    server.use(
      rest.get('/api/schemas/available', (req, res, ctx) => {
        return res(ctx.json({ data: ['schema1', 'schema2'] }));
      }),
      rest.get('/api/schemas', (req, res, ctx) => {
        return res(ctx.json({
          data: { schema1: 'approved', schema2: 'available' }
        }));
      }),
      rest.get('/api/schema/schema1', (req, res, ctx) => {
        return res(ctx.json({
          name: 'schema1',
          state: 'approved',
          fields: { name: { field_type: 'String' } }
        }));
      })
    );

    const schemas = await schemaClient.getApprovedSchemas();

    expect(schemas).toHaveLength(1);
    expect(schemas[0].name).toBe('schema1');
    expect(schemas[0].state).toBe('approved');
  });

  test('should handle schema approval with proper validation', async () => {
    server.use(
      rest.post('/api/schema/test_schema/approve', (req, res, ctx) => {
        return res(ctx.json({
          success: true,
          data: {
            schema: { name: 'test_schema', state: 'approved' }
          }
        }));
      })
    );

    const result = await schemaClient.approveSchema('test_schema');

    expect(result.success).toBe(true);
    expect(result.data.schema.state).toBe('approved');
  });

  test('should retry failed requests with exponential backoff', async () => {
    let attemptCount = 0;
    
    server.use(
      rest.get('/api/schemas', (req, res, ctx) => {
        attemptCount++;
        if (attemptCount < 3) {
          return res(ctx.status(500));
        }
        return res(ctx.json({ data: {} }));
      })
    );

    const result = await schemaClient.getSchemas();
    
    expect(attemptCount).toBe(3);
    expect(result).toBeDefined();
  });

  test('should handle timeout errors appropriately', async () => {
    server.use(
      rest.get('/api/schemas', (req, res, ctx) => {
        return res(ctx.delay(35000)); // Longer than timeout
      })
    );

    await expect(schemaClient.getSchemas()).rejects.toThrow('Operation timed out');
  });
});
```

## Testing Utilities

### Mock Providers

**Redux Provider Wrapper**:
```javascript
// src/test/utils/testProviders.jsx
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { schemaSlice } from '../../store/schemaSlice';
import { authSlice } from '../../store/authSlice';

export const createTestStore = (initialState = {}) => {
  return configureStore({
    reducer: {
      schemas: schemaSlice.reducer,
      auth: authSlice.reducer
    },
    preloadedState: initialState
  });
};

export const renderWithProviders = (ui, options = {}) => {
  const {
    initialState = {},
    store = createTestStore(initialState),
    ...renderOptions
  } = options;

  const Wrapper = ({ children }) => (
    <Provider store={store}>{children}</Provider>
  );

  return {
    store,
    ...render(ui, { wrapper: Wrapper, ...renderOptions })
  };
};
```

### API Mocks

**MSW Handlers**:
```javascript
// src/test/mocks/handlers.js
import { rest } from 'msw';

export const handlers = [
  // Schema endpoints
  rest.get('/api/schemas/available', (req, res, ctx) => {
    return res(ctx.json({
      data: ['user_profiles', 'time_series', 'events']
    }));
  }),

  rest.get('/api/schemas', (req, res, ctx) => {
    return res(ctx.json({
      data: {
        user_profiles: 'approved',
        time_series: 'approved',
        events: 'available'
      }
    }));
  }),

  rest.get('/api/schema/:schemaName', (req, res, ctx) => {
    const { schemaName } = req.params;
    return res(ctx.json({
      name: schemaName,
      state: 'approved',
      fields: {
        id: { field_type: 'String' },
        created_at: { field_type: 'String' }
      }
    }));
  }),

  // Schema operations
  rest.post('/api/schema/:schemaName/approve', (req, res, ctx) => {
    const { schemaName } = req.params;
    return res(ctx.json({
      success: true,
      data: {
        schema: { name: schemaName, state: 'approved' }
      }
    }));
  }),

  // Mutation endpoints
  rest.post('/api/mutation', (req, res, ctx) => {
    return res(ctx.json({
      success: true,
      data: { id: 'mutation_123' }
    }));
  }),

  // Query endpoints
  rest.post('/api/query', (req, res, ctx) => {
    return res(ctx.json({
      success: true,
      data: [
        { id: '1', name: 'John' },
        { id: '2', name: 'Jane' }
      ]
    }));
  })
];
```

### Test Fixtures

**Schema Fixtures**:
```javascript
// src/test/fixtures/schemaFixtures.js
export const mockApprovedSchema = {
  name: 'user_profiles',
  state: 'approved',
  fields: {
    id: { field_type: 'String' },
    name: { field_type: 'String' },
    email: { field_type: 'String' },
    age: { field_type: 'Number' }
  }
};

export const mockRangeSchema = {
  name: 'time_series',
  state: 'approved',
  fields: {
    timestamp: { field_type: 'Range' },
    value: { field_type: 'Range' },
    metadata: { field_type: 'Range' }
  },
  schema_type: {
    Range: { range_key: 'timestamp' }
  }
};

export const mockAvailableSchema = {
  name: 'events',
  state: 'available',
  fields: {
    event_id: { field_type: 'String' },
    event_type: { field_type: 'String' }
  }
};

export const createMockSchemaList = (count = 3) => {
  return Array.from({ length: count }, (_, i) => ({
    name: `schema_${i}`,
    state: i % 2 === 0 ? 'approved' : 'available',
    fields: {
      id: { field_type: 'String' },
      data: { field_type: 'String' }
    }
  }));
};
```

### Custom Render Utilities

**Hook Testing Utilities**:
```javascript
// src/test/utils/hookUtils.js
import { renderHook } from '@testing-library/react';
import { Provider } from 'react-redux';
import { createTestStore } from './testProviders';

export const renderHookWithProviders = (hook, options = {}) => {
  const {
    initialState = {},
    store = createTestStore(initialState),
    ...renderOptions
  } = options;

  const wrapper = ({ children }) => (
    <Provider store={store}>{children}</Provider>
  );

  return {
    store,
    ...renderHook(hook, { wrapper, ...renderOptions })
  };
};

export const createMockHookResult = (overrides = {}) => ({
  data: [],
  isLoading: false,
  error: null,
  refetch: vi.fn(),
  ...overrides
});
```

## Testing Patterns

### Testing Async Operations

```javascript
// Testing async hooks with waitFor
test('should handle async data fetching', async () => {
  const { result } = renderHookWithProviders(() => useApprovedSchemas());

  expect(result.current.isLoading).toBe(true);

  await waitFor(() => {
    expect(result.current.isLoading).toBe(false);
  });

  expect(result.current.approvedSchemas).toHaveLength(2);
});
```

### Testing Error States

```javascript
// Testing error handling
test('should handle API errors gracefully', async () => {
  server.use(
    rest.get('/api/schemas/available', (req, res, ctx) => {
      return res(ctx.status(500), ctx.json({ error: 'Server error' }));
    })
  );

  const { result } = renderHookWithProviders(() => useApprovedSchemas());

  await waitFor(() => {
    expect(result.current.error).toBeTruthy();
  });

  expect(result.current.error).toContain('Failed to fetch');
});
```

### Testing User Interactions

```javascript
// Testing complex user interactions
test('should handle complete mutation workflow', async () => {
  const user = userEvent.setup();
  
  renderWithProviders(<MutationForm />);

  // Select schema
  await user.click(screen.getByLabelText('Select Schema'));
  await user.click(screen.getByText('user_profiles'));

  // Fill form
  await user.type(screen.getByLabelText('Name'), 'John Doe');
  await user.type(screen.getByLabelText('Email'), 'john@example.com');

  // Submit
  await user.click(screen.getByRole('button', { name: /submit/i }));

  await waitFor(() => {
    expect(screen.getByText('Mutation successful')).toBeInTheDocument();
  });
});
```

## Coverage Requirements

### Minimum Coverage Thresholds

- **Lines**: 80%
- **Functions**: 80%
- **Branches**: 80%
- **Statements**: 80%

### Coverage Reports

```bash
# Generate coverage report
npm run test:coverage

# View HTML report
open coverage/index.html

# Check coverage in CI
npm run test:coverage:ci
```

### Coverage Exceptions

Files exempt from coverage requirements:
- Test files (`*.test.js`, `*.spec.js`)
- Configuration files (`*.config.js`)
- Type definitions (`*.d.ts`)
- Mock files and fixtures

## Best Practices

### 1. Test Organization

```javascript
// Good: Descriptive test names
describe('useApprovedSchemas', () => {
  describe('when schemas are loading', () => {
    test('should return loading state as true', () => {
      // Test implementation
    });
  });

  describe('when schemas are loaded', () => {
    test('should return approved schemas only', () => {
      // Test implementation
    });

    test('should provide schema validation functions', () => {
      // Test implementation
    });
  });
});
```

### 2. Test Data Management

```javascript
// Good: Use factories for test data
const createMockSchema = (overrides = {}) => ({
  name: 'default_schema',
  state: 'approved',
  fields: {},
  ...overrides
});

// Use in tests
const approvedSchema = createMockSchema({ state: 'approved' });
const blockedSchema = createMockSchema({ state: 'blocked' });
```

### 3. Assertion Quality

```javascript
// Good: Specific assertions
expect(result.current.approvedSchemas).toHaveLength(2);
expect(result.current.approvedSchemas[0].name).toBe('user_profiles');

// Avoid: Generic assertions
expect(result.current.approvedSchemas).toBeTruthy();
```

### 4. Test Independence

```javascript
// Good: Each test sets up its own state
beforeEach(() => {
  server.resetHandlers();
});

test('should handle success case', () => {
  server.use(
    rest.get('/api/schemas', (req, res, ctx) => {
      return res(ctx.json({ data: mockSchemas }));
    })
  );
  // Test implementation
});
```

### 5. Error Testing

```javascript
// Good: Test error scenarios
test('should handle network errors', async () => {
  server.use(
    rest.get('/api/schemas', (req, res, ctx) => {
      return res.networkError('Failed to connect');
    })
  );

  const { result } = renderHookWithProviders(() => useApprovedSchemas());

  await waitFor(() => {
    expect(result.current.error).toContain('Network error');
  });
});
```

## Troubleshooting

### Common Issues

#### 1. Tests Timing Out

**Problem**: Async tests hanging indefinitely.
**Solution**: Use `waitFor` with proper timeout and cleanup.

```javascript
// Good
await waitFor(() => {
  expect(result.current.isLoading).toBe(false);
}, { timeout: 5000 });
```

#### 2. MSW Handlers Not Working

**Problem**: API mocks not intercepting requests.
**Solution**: Ensure server is properly set up and handlers match exact URLs.

```javascript
// Check handler URL patterns
rest.get('/api/schemas/available', handler) // Exact match
rest.get('/api/schema/:id', handler)        // Parameter match
```

#### 3. Redux State Not Updating

**Problem**: Component tests not reflecting state changes.
**Solution**: Use proper providers and wait for state updates.

```javascript
// Ensure proper provider setup
const { store } = renderWithProviders(<Component />, {
  initialState: { schemas: mockState }
});
```

#### 4. Memory Leaks in Tests

**Problem**: Tests using too much memory.
**Solution**: Proper cleanup and mocking.

```javascript
afterEach(() => {
  cleanup();
  server.resetHandlers();
  vi.clearAllMocks();
});
```

### Debugging Tests

```javascript
// Debug test state
test('debug example', () => {
  const { result } = renderHookWithProviders(() => useApprovedSchemas());
  
  // Debug hook result
  console.log('Hook result:', result.current);
  
  // Debug Redux state
  console.log('Store state:', store.getState());
});
```

### Performance Testing

```javascript
// Test rendering performance
test('should render efficiently', () => {
  const startTime = performance.now();
  
  renderWithProviders(<ExpensiveComponent />);
  
  const endTime = performance.now();
  expect(endTime - startTime).toBeLessThan(100); // 100ms threshold
});
```

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 2.0.0 | 2025-06-24 | Comprehensive testing guide for React simplification |

For additional testing resources, see the [Architecture Guide](./architecture.md) and [Migration Guide](./migration.md).