# Developer Usage Guide - API Clients

**Version:** 1.0  
**Date:** June 28, 2025  
**Part of:** API-STD-1 Product Backlog Item  

## Getting Started

This guide provides practical examples and best practices for using the unified API client architecture in React components, hooks, and Redux stores.

### Installation and Setup

All API clients are available through centralized exports:

```typescript
// Import specific clients
import { schemaClient, securityClient, systemClient } from '../api/clients';

// Or import the client classes for custom instances
import { UnifiedSchemaClient, UnifiedSecurityClient } from '../api/clients';

// Import types for full type safety
import type { 
  EnhancedApiResponse, 
  SchemaData, 
  SystemStatus 
} from '../api/clients';
```

### Basic Usage Pattern

All API clients follow the same consistent pattern:

```typescript
const response = await client.methodName(params, options);

if (response.success) {
  const data = response.data; // Fully typed response data
  const metadata = response.meta; // Request metadata (ID, timestamp, cache info)
} else {
  // Handle error (response.error will contain error details)
  console.error('API Error:', response.error);
}
```

## Using API Clients in Components

### React Functional Components

#### Basic Data Fetching

```typescript
import React, { useState, useEffect } from 'react';
import { schemaClient } from '../api/clients';
import type { SchemaData } from '../api/clients';

const SchemaList: React.FC = () => {
  const [schemas, setSchemas] = useState<SchemaData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadSchemas = async () => {
      try {
        setLoading(true);
        const response = await schemaClient.getSchemas();
        
        if (response.success) {
          setSchemas(response.data);
        } else {
          setError('Failed to load schemas');
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    };

    loadSchemas();
  }, []);

  if (loading) return <div>Loading schemas...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <ul>
      {schemas.map(schema => (
        <li key={schema.name}>
          {schema.name} - {schema.state}
        </li>
      ))}
    </ul>
  );
};
```

#### With Error Handling Best Practices

```typescript
import React, { useState, useEffect } from 'react';
import { systemClient } from '../api/clients';
import { isNetworkError, isAuthenticationError } from '../api/core/errors';
import type { SystemStatusResponse } from '../api/clients';

const SystemStatus: React.FC = () => {
  const [status, setStatus] = useState<SystemStatusResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadStatus = async () => {
      try {
        const response = await systemClient.getSystemStatus();
        
        if (response.success) {
          setStatus(response.data);
          setError(null);
        }
      } catch (err) {
        // Use built-in error handling utilities
        if (isNetworkError(err)) {
          setError('Network connection failed. Please check your internet connection.');
        } else if (isAuthenticationError(err)) {
          setError('Authentication required. Please log in.');
        } else {
          setError(err.toUserMessage ? err.toUserMessage() : 'An error occurred');
        }
      }
    };

    loadStatus();
    
    // Refresh status every 30 seconds
    const interval = setInterval(loadStatus, 30000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div>
      {error && <div className="error">{error}</div>}
      {status && (
        <div>
          <h3>System Status</h3>
          <p>Database: {status.database_status}</p>
          <p>Memory Usage: {status.memory_usage_mb} MB</p>
          <p>Uptime: {status.uptime_seconds}s</p>
        </div>
      )}
    </div>
  );
};
```

### Custom Hooks

#### Schema Management Hook

```typescript
import { useState, useEffect, useCallback } from 'react';
import { schemaClient } from '../api/clients';
import type { SchemaData, EnhancedApiResponse } from '../api/clients';

interface UseSchemaResult {
  schemas: SchemaData[];
  loading: boolean;
  error: string | null;
  refetch: () => Promise<void>;
  approveSchema: (name: string) => Promise<boolean>;
  blockSchema: (name: string) => Promise<boolean>;
}

export const useSchemas = (): UseSchemaResult => {
  const [schemas, setSchemas] = useState<SchemaData[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchSchemas = useCallback(async () => {
    try {
      setLoading(true);
      const response = await schemaClient.getSchemas();
      
      if (response.success) {
        setSchemas(response.data);
        setError(null);
      } else {
        setError('Failed to load schemas');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, []);

  const approveSchema = useCallback(async (name: string): Promise<boolean> => {
    try {
      const response = await schemaClient.approveSchema(name);
      if (response.success) {
        await fetchSchemas(); // Refresh the list
        return true;
      }
      return false;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to approve schema');
      return false;
    }
  }, [fetchSchemas]);

  const blockSchema = useCallback(async (name: string): Promise<boolean> => {
    try {
      const response = await schemaClient.blockSchema(name);
      if (response.success) {
        await fetchSchemas(); // Refresh the list
        return true;
      }
      return false;
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to block schema');
      return false;
    }
  }, [fetchSchemas]);

  useEffect(() => {
    fetchSchemas();
  }, [fetchSchemas]);

  return {
    schemas,
    loading,
    error,
    refetch: fetchSchemas,
    approveSchema,
    blockSchema
  };
};
```

#### Transform Queue Hook

```typescript
import { useState, useEffect, useCallback } from 'react';
import { transformClient } from '../api/clients';
import type { Transform, QueueInfo } from '../api/clients';

export const useTransformQueue = () => {
  const [queue, setQueue] = useState<QueueInfo | null>(null);
  const [transforms, setTransforms] = useState<Transform[]>([]);
  const [loading, setLoading] = useState(false);

  const refreshQueue = useCallback(async () => {
    setLoading(true);
    try {
      const [queueResponse, transformsResponse] = await Promise.all([
        transformClient.getQueue(),
        transformClient.getTransforms()
      ]);

      if (queueResponse.success) {
        setQueue(queueResponse.data);
      }
      
      if (transformsResponse.success) {
        setTransforms(transformsResponse.data.transforms);
      }
    } catch (err) {
      console.error('Failed to refresh queue:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  const addToQueue = useCallback(async (transformId: string) => {
    try {
      const response = await transformClient.addToQueue(transformId);
      if (response.success) {
        await refreshQueue();
        return true;
      }
      return false;
    } catch (err) {
      console.error('Failed to add to queue:', err);
      return false;
    }
  }, [refreshQueue]);

  const removeFromQueue = useCallback(async (transformId: string) => {
    try {
      const response = await transformClient.removeFromQueue(transformId);
      if (response.success) {
        await refreshQueue();
        return true;
      }
      return false;
    } catch (err) {
      console.error('Failed to remove from queue:', err);
      return false;
    }
  }, [refreshQueue]);

  useEffect(() => {
    refreshQueue();
  }, [refreshQueue]);

  return {
    queue,
    transforms,
    loading,
    refreshQueue,
    addToQueue,
    removeFromQueue
  };
};
```

### Redux Integration

#### Schema Slice

```typescript
import { createSlice, createAsyncThunk } from '@reduxjs/toolkit';
import { schemaClient } from '../api/clients';
import type { SchemaData } from '../api/clients';

interface SchemaState {
  schemas: SchemaData[];
  loading: boolean;
  error: string | null;
}

const initialState: SchemaState = {
  schemas: [],
  loading: false,
  error: null
};

// Async thunks using API clients
export const fetchSchemas = createAsyncThunk(
  'schemas/fetchSchemas',
  async (_, { rejectWithValue }) => {
    try {
      const response = await schemaClient.getSchemas();
      if (response.success) {
        return response.data;
      } else {
        return rejectWithValue('Failed to fetch schemas');
      }
    } catch (err) {
      return rejectWithValue(err instanceof Error ? err.message : 'Unknown error');
    }
  }
);

export const approveSchema = createAsyncThunk(
  'schemas/approveSchema',
  async (schemaName: string, { rejectWithValue }) => {
    try {
      const response = await schemaClient.approveSchema(schemaName);
      if (response.success) {
        return schemaName;
      } else {
        return rejectWithValue('Failed to approve schema');
      }
    } catch (err) {
      return rejectWithValue(err instanceof Error ? err.message : 'Unknown error');
    }
  }
);

const schemaSlice = createSlice({
  name: 'schemas',
  initialState,
  reducers: {
    clearError: (state) => {
      state.error = null;
    }
  },
  extraReducers: (builder) => {
    builder
      .addCase(fetchSchemas.pending, (state) => {
        state.loading = true;
        state.error = null;
      })
      .addCase(fetchSchemas.fulfilled, (state, action) => {
        state.loading = false;
        state.schemas = action.payload;
      })
      .addCase(fetchSchemas.rejected, (state, action) => {
        state.loading = false;
        state.error = action.payload as string;
      })
      .addCase(approveSchema.fulfilled, (state, action) => {
        // Update the approved schema in the state
        const schema = state.schemas.find(s => s.name === action.payload);
        if (schema) {
          schema.state = 'approved';
        }
      });
  }
});

export const { clearError } = schemaSlice.actions;
export default schemaSlice.reducer;
```

## Code Examples for Common Operations

### Schema Operations

```typescript
import { schemaClient } from '../api/clients';

// Get all schemas
const allSchemas = await schemaClient.getSchemas();

// Get schemas by state
const approvedSchemas = await schemaClient.getSchemasByState('approved');

// Get specific schema
const userSchema = await schemaClient.getSchema('users');

// Schema state management
await schemaClient.approveSchema('users');
await schemaClient.blockSchema('temp_data');

// Schema loading/unloading
await schemaClient.loadSchema('users');
await schemaClient.unloadSchema('temp_data');

// Validation
const validation = await schemaClient.validateSchemaForOperation(
  'users', 
  'mutation'
);
```

### Security Operations

```typescript
import { securityClient } from '../api/clients';

// Message verification
const signedMessage = {
  payload: 'base64-encoded-data',
  signature: 'base64-signature',
  public_key_id: 'key-id',
  timestamp: Math.floor(Date.now() / 1000),
  nonce: 'random-string'
};

const verification = await securityClient.verifyMessage(signedMessage);
if (verification.success && verification.data.is_valid) {
  console.log('Message verified successfully');
}

// Key management
const keyRequest = securityClient.createKeyRegistrationRequest(
  'base64-public-key',
  'user-id',
  ['read', 'write'],
  { expiresAt: Date.now() / 1000 + 86400 } // Expires in 24 hours
);

const registration = await securityClient.registerPublicKey(keyRequest);

// Get system public key
const systemKey = await securityClient.getSystemPublicKey();
```

### Transform Operations

```typescript
import { transformClient } from '../api/clients';

// Get all transforms
const transforms = await transformClient.getTransforms();

// Queue management
const queueInfo = await transformClient.getQueue();

// Add transform to queue
await transformClient.addToQueue('transform-123');

// Remove from queue
await transformClient.removeFromQueue('transform-123');

// Get specific transform
const transform = await transformClient.getTransform('transform-123');
```

### Ingestion Operations

```typescript
import { ingestionClient } from '../api/clients';

// Check ingestion status
const status = await ingestionClient.getStatus();

// Validate data structure
const validationResult = await ingestionClient.validateData({
  users: [
    { name: 'John', age: 30 },
    { name: 'Jane', age: 25 }
  ]
});

// Process ingestion with AI
const result = await ingestionClient.processIngestion(
  { users: [{ name: 'John', age: 30 }] },
  {
    autoExecute: true,
    trustDistance: 1,
    pubKey: 'user-public-key'
  }
);

// Configuration management
const config = ingestionClient.createOpenRouterConfig(
  'your-api-key',
  'anthropic/claude-3.5-sonnet',
  { maxTokens: 4000, temperature: 0.7 }
);

await ingestionClient.saveConfig(config);
```

### System Operations

```typescript
import { systemClient } from '../api/clients';

// Get system status
const status = await systemClient.getSystemStatus();

// Get system logs
const logs = await systemClient.getLogs();

// Create log stream for real-time updates
const stream = systemClient.createLogStream((logEntry) => {
  console.log('New log:', logEntry);
});

// Database reset (dangerous operation)
const confirmation = confirm('Are you sure you want to reset the database?');
if (confirmation) {
  await systemClient.resetDatabase(true);
}
```

## Error Handling Best Practices

### Comprehensive Error Handling

```typescript
import { 
  schemaClient,
  isApiError,
  isNetworkError,
  isTimeoutError,
  isAuthenticationError,
  isSchemaStateError,
  isValidationError,
  isRateLimitError
} from '../api/clients';

const handleSchemaOperation = async (schemaName: string) => {
  try {
    const response = await schemaClient.getSchema(schemaName);
    
    if (response.success) {
      return response.data;
    }
  } catch (error) {
    // Use type guards for specific error handling
    if (isAuthenticationError(error)) {
      // Redirect to login or refresh auth
      redirectToLogin();
      return null;
    }
    
    if (isSchemaStateError(error)) {
      // Handle schema state violations
      showMessage(`Schema "${error.schemaName}" is ${error.currentState} and cannot be accessed for ${error.operation}`);
      return null;
    }
    
    if (isValidationError(error)) {
      // Handle validation errors
      const errors = error.validationErrors;
      showValidationErrors(errors);
      return null;
    }
    
    if (isNetworkError(error)) {
      // Handle network issues
      showRetryDialog('Network error occurred. Would you like to retry?');
      return null;
    }
    
    if (isTimeoutError(error)) {
      // Handle timeouts
      showMessage(`Request timed out after ${error.timeoutMs}ms. Please try again.`);
      return null;
    }
    
    if (isRateLimitError(error)) {
      // Handle rate limiting
      const retryAfter = error.retryAfter || 60;
      showMessage(`Rate limit exceeded. Try again in ${retryAfter} seconds.`);
      return null;
    }
    
    // Generic API error
    if (isApiError(error)) {
      showMessage(error.toUserMessage());
      return null;
    }
    
    // Unknown error
    console.error('Unknown error:', error);
    showMessage('An unexpected error occurred. Please try again.');
    return null;
  }
};
```

### Error Boundary Integration

```typescript
import React from 'react';
import { isApiError } from '../api/core/errors';

interface ErrorBoundaryState {
  hasError: boolean;
  error: Error | null;
}

class ApiErrorBoundary extends React.Component<
  React.PropsWithChildren<{}>,
  ErrorBoundaryState
> {
  constructor(props: React.PropsWithChildren<{}>) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo) {
    // Log API errors with additional context
    if (isApiError(error)) {
      console.error('API Error caught by boundary:', {
        message: error.message,
        status: error.status,
        requestId: error.requestId,
        timestamp: error.timestamp,
        errorInfo
      });
    } else {
      console.error('Non-API error caught by boundary:', error, errorInfo);
    }
  }

  render() {
    if (this.state.hasError && this.state.error) {
      if (isApiError(this.state.error)) {
        return (
          <div className="error-boundary">
            <h2>API Error</h2>
            <p>{this.state.error.toUserMessage()}</p>
            <button onClick={() => this.setState({ hasError: false, error: null })}>
              Try Again
            </button>
          </div>
        );
      }
      
      return (
        <div className="error-boundary">
          <h2>Something went wrong</h2>
          <p>An unexpected error occurred. Please refresh the page.</p>
        </div>
      );
    }

    return this.props.children;
  }
}
```

## Guidelines for Adding New API Operations

### Extending Existing Clients

When adding new operations to existing clients:

1. **Add the method to the client class:**

```typescript
// In schemaClient.ts
async getSchemaMetrics(schemaName: string): Promise<EnhancedApiResponse<SchemaMetrics>> {
  return this.client.get<SchemaMetrics>(
    `/schemas/${schemaName}/metrics`,
    {
      requiresAuth: true,
      timeout: API_TIMEOUTS.STANDARD,
      retries: API_RETRIES.STANDARD,
      cacheable: true,
      cacheTtl: API_CACHE_TTL.SCHEMA_DATA
    }
  );
}
```

2. **Add TypeScript interfaces:**

```typescript
// Add to the client file or types file
export interface SchemaMetrics {
  schemaName: string;
  recordCount: number;
  sizeBytes: number;
  lastUpdated: string;
  queryCount: number;
  mutationCount: number;
}
```

3. **Add validation if needed:**

```typescript
validateSchemaName(schemaName: string): { isValid: boolean; error?: string } {
  if (!schemaName || schemaName.trim().length === 0) {
    return { isValid: false, error: 'Schema name is required' };
  }
  
  if (!/^[a-zA-Z_][a-zA-Z0-9_]*$/.test(schemaName)) {
    return { isValid: false, error: 'Schema name must be a valid identifier' };
  }
  
  return { isValid: true };
}
```

4. **Export the new method:**

```typescript
// In the client file
export const getSchemaMetrics = schemaClient.getSchemaMetrics.bind(schemaClient);

// In index.ts
export { getSchemaMetrics } from './schemaClient';
```

### Creating New Clients

For completely new domains, create a new client:

1. **Create the client file:**

```typescript
// clients/analyticsClient.ts
import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES } from '../../constants/api';
import type { EnhancedApiResponse } from '../core/types';

export interface AnalyticsData {
  // Define your interfaces
}

export class UnifiedAnalyticsClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: true,
      enableLogging: true,
      enableMetrics: true
    });
  }

  async getAnalytics(): Promise<EnhancedApiResponse<AnalyticsData>> {
    return this.client.get<AnalyticsData>(
      API_ENDPOINTS.ANALYTICS,
      {
        requiresAuth: true,
        timeout: API_TIMEOUTS.STANDARD,
        retries: API_RETRIES.STANDARD
      }
    );
  }
}

export const analyticsClient = new UnifiedAnalyticsClient();
export default analyticsClient;
```

2. **Add endpoints:**

```typescript
// In endpoints.ts
export const API_ENDPOINTS = {
  // ... existing endpoints
  ANALYTICS: '/analytics',
  ANALYTICS_SUMMARY: '/analytics/summary'
};
```

3. **Add to index.ts:**

```typescript
// In clients/index.ts
export {
  analyticsClient,
  UnifiedAnalyticsClient
} from './analyticsClient';
```

## Migration Patterns from Direct fetch()

### Before: Direct fetch() Usage

```typescript
// Old pattern - repeated boilerplate
const fetchSchemas = async () => {
  try {
    const response = await fetch('/api/schemas', {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${getAuthToken()}`
      }
    });

    if (!response.ok) {
      if (response.status === 401) {
        redirectToLogin();
        return;
      }
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const data = await response.json();
    return data.schemas || data;
  } catch (error) {
    if (error instanceof TypeError) {
      // Network error
      showErrorMessage('Network connection failed');
    } else {
      showErrorMessage(error.message);
    }
    throw error;
  }
};
```

### After: Unified Client Usage

```typescript
// New pattern - clean and standardized
const fetchSchemas = async () => {
  try {
    const response = await schemaClient.getSchemas();
    
    if (response.success) {
      return response.data; // Fully typed SchemaData[]
    }
  } catch (error) {
    // Automatic error handling with type guards
    if (isAuthenticationError(error)) {
      redirectToLogin();
      return;
    }
    
    showErrorMessage(error.toUserMessage());
    throw error;
  }
};
```

### Migration Steps

1. **Identify fetch() calls** in your components
2. **Import the appropriate client** for the API domain
3. **Replace fetch() with client method** calls
4. **Update error handling** to use error type guards
5. **Add TypeScript types** for response data
6. **Remove manual header management** (handled automatically)
7. **Remove manual retry logic** (handled by client)

## Testing Strategies

### Unit Testing API Client Methods

```typescript
// __tests__/schemaClient.test.ts
import { UnifiedSchemaClient } from '../schemaClient';
import { ApiClient } from '../core/client';

// Mock the core client
const mockClient = {
  get: jest.fn(),
  post: jest.fn(),
  put: jest.fn(),
  delete: jest.fn()
} as unknown as ApiClient;

const schemaClient = new UnifiedSchemaClient(mockClient);

describe('UnifiedSchemaClient', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  test('getSchemas calls correct endpoint', async () => {
    const mockResponse = {
      success: true,
      data: [{ name: 'users', state: 'approved' }]
    };
    
    (mockClient.get as jest.Mock).mockResolvedValue(mockResponse);

    const result = await schemaClient.getSchemas();

    expect(mockClient.get).toHaveBeenCalledWith('/schemas', {
      requiresAuth: false,
      timeout: 8000,
      retries: 2,
      cacheable: true,
      cacheTtl: 300000
    });
    
    expect(result).toEqual(mockResponse);
  });

  test('approveSchema handles errors correctly', async () => {
    const mockError = new Error('Network error');
    (mockClient.put as jest.Mock).mockRejectedValue(mockError);

    await expect(schemaClient.approveSchema('users')).rejects.toThrow('Network error');
  });
});
```

### Integration Testing with Components

```typescript
// __tests__/SchemaList.integration.test.tsx
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { SchemaList } from '../SchemaList';
import { schemaClient } from '../api/clients';

// Mock the entire client
jest.mock('../api/clients', () => ({
  schemaClient: {
    getSchemas: jest.fn()
  }
}));

const mockSchemaClient = schemaClient as jest.Mocked<typeof schemaClient>;

describe('SchemaList Integration', () => {
  test('displays schemas when loaded successfully', async () => {
    mockSchemaClient.getSchemas.mockResolvedValue({
      success: true,
      data: [
        { name: 'users', state: 'approved' },
        { name: 'posts', state: 'available' }
      ]
    });

    render(<SchemaList />);

    await waitFor(() => {
      expect(screen.getByText('users - approved')).toBeInTheDocument();
      expect(screen.getByText('posts - available')).toBeInTheDocument();
    });
  });

  test('displays error message when loading fails', async () => {
    mockSchemaClient.getSchemas.mockRejectedValue(new Error('Network error'));

    render(<SchemaList />);

    await waitFor(() => {
      expect(screen.getByText(/Error: Network error/)).toBeInTheDocument();
    });
  });
});
```

### Mocking Strategies

#### Complete Client Mocking

```typescript
// __mocks__/api/clients.ts
export const mockSchemaClient = {
  getSchemas: jest.fn(),
  getSchema: jest.fn(),
  approveSchema: jest.fn(),
  blockSchema: jest.fn()
};

export const mockSecurityClient = {
  verifyMessage: jest.fn(),
  registerPublicKey: jest.fn(),
  getSystemPublicKey: jest.fn()
};

export const schemaClient = mockSchemaClient;
export const securityClient = mockSecurityClient;
```

#### Test Utilities

```typescript
// test-utils/apiMocks.ts
import type { EnhancedApiResponse } from '../api/core/types';

export const createMockResponse = <T>(
  data: T,
  success: boolean = true
): EnhancedApiResponse<T> => ({
  success,
  data,
  status: success ? 200 : 500,
  meta: {
    requestId: 'test-request-id',
    timestamp: Date.now(),
    cached: false,
    fromCache: false
  }
});

export const createMockError = (message: string, status: number = 500) => {
  const error = new Error(message);
  (error as any).status = status;
  (error as any).toUserMessage = () => message;
  return error;
};
```

### Testing Error Scenarios

```typescript
// __tests__/errorHandling.test.ts
import { schemaClient } from '../api/clients';
import { 
  ApiError, 
  NetworkError, 
  AuthenticationError 
} from '../api/core/errors';

describe('Error Handling', () => {
  test('handles authentication errors', async () => {
    const authError = new AuthenticationError('Token expired');
    jest.spyOn(schemaClient, 'getSchemas').mockRejectedValue(authError);

    try {
      await schemaClient.getSchemas();
    } catch (error) {
      expect(error).toBeInstanceOf(AuthenticationError);
      expect(error.status).toBe(401);
      expect(error.toUserMessage()).toBe('Authentication required. Please ensure you are properly authenticated.');
    }
  });

  test('handles network errors', async () => {
    const networkError = new NetworkError('Connection failed');
    jest.spyOn(schemaClient, 'getSchemas').mockRejectedValue(networkError);

    try {
      await schemaClient.getSchemas();
    } catch (error) {
      expect(error).toBeInstanceOf(NetworkError);
      expect(error.isNetworkError).toBe(true);
      expect(error.toUserMessage()).toBe('Network connection failed. Please check your internet connection.');
    }
  });
});
```

## Performance Optimization Tips

### Efficient Data Fetching

```typescript
// Use batch operations for multiple requests
const [schemas, status, transforms] = await Promise.all([
  schemaClient.getSchemas(),
  systemClient.getSystemStatus(),
  transformClient.getTransforms()
]);

// Or use the core client's batch functionality
const responses = await apiClient.batch([
  { id: 'schemas', method: 'GET', url: '/schemas' },
  { id: 'status', method: 'GET', url: '/system/status' },
  { id: 'transforms', method: 'GET', url: '/transforms' }
]);
```

### Cache Management

```typescript
// Clear cache when needed
schemaClient.clearCache();

// Get cache statistics
const stats = schemaClient.getCacheStats();
console.log(`Cache hit rate: ${stats.hitRate * 100}%`);

// Custom cache TTL for specific requests
const response = await schemaClient.getSchemas({
  cacheTtl: 60000 // Cache for 1 minute instead of default 5 minutes
});
```

### Request Optimization

```typescript
// Use AbortController for cancellable requests
const controller = new AbortController();

const response = await schemaClient.getSchemas({
  abortSignal: controller.signal
});

// Cancel if component unmounts
useEffect(() => {
  return () => controller.abort();
}, []);
```

## Conclusion

The unified API client architecture provides a consistent, type-safe, and maintainable approach to API communication. By following these patterns and best practices, developers can build robust React applications with reliable data fetching, comprehensive error handling, and excellent performance characteristics.

The key benefits include:
- **Reduced boilerplate** through standardized patterns
- **Type safety** with comprehensive TypeScript support
- **Automatic error handling** with user-friendly messages
- **Performance optimizations** through caching and request deduplication
- **Consistent patterns** across all API operations
- **Easy testing** with built-in mocking strategies

Remember to always handle errors appropriately, use the type guards for specific error types, and leverage the caching system for better performance.