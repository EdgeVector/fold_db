/**
 * @fileoverview Consolidated Testing Utilities
 * TASK-010: Test Suite Fixes and Validation for PBI-REACT-SIMPLIFY-001
 * 
 * Unified testing utilities that eliminate duplication between testStore.jsx
 * and testingUtilities.jsx files. Provides comprehensive testing support
 * for Redux store setup, component rendering, API mocking, and validation.
 * 
 * @module testUtilities
 * @since 2.1.0
 */

import React from 'react';
import { render, renderHook } from '@testing-library/react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import {
  TEST_TIMEOUT_DEFAULT_MS,
  MOCK_API_DELAY_MS,
  COVERAGE_THRESHOLD_PERCENT,
  TEST_VALIDATION_BATCH_SIZE,
  INTEGRATION_TEST_RETRY_COUNT
} from '../config/constants';
import authReducer from '../../store/authSlice';
import schemaReducer from '../../store/schemaSlice';
import { SCHEMA_STATES } from '../../constants/schemas';

export { SCHEMA_STATES };

/**
 * Creates a test store with optional initial state
 * Combines the best features from both previous implementations
 * @param {Object} preloadedState - Initial state for the store
 * @returns {Object} Configured test store
 */
export function createTestStore(preloadedState = {}) {
  const defaultState = {
    auth: {
      isAuthenticated: false,
      privateKey: null,
      systemKeyId: null,
      publicKey: null,
      loading: false,
      error: null
    },
    schemas: {
      schemas: {},  // Match actual store structure - object indexed by schema ID
      loading: {
        fetch: false,
        operations: {}
      },
      errors: {
        fetch: null,
        operations: {}
      },
      lastFetched: null,
      cache: {
        ttl: 300000,
        version: '2.1.0',
        lastUpdated: null
      },
      activeSchema: null
    }
  };

  return configureStore({
    reducer: {
      auth: authReducer,
      schemas: schemaReducer,
    },
    preloadedState: {
      ...defaultState,
      ...preloadedState
    },
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({
        serializableCheck: {
          // Ignore these action types
          ignoredActions: [
            'auth/validatePrivateKey/fulfilled',
            'auth/setPrivateKey',
            'schemas/fetchSchemas/fulfilled',
            'schemas/approveSchema/fulfilled',
            'schemas/blockSchema/fulfilled',
            'schemas/unloadSchema/fulfilled',
            'schemas/loadSchema/fulfilled',
            'persist/PERSIST'
          ],
          // Ignore these field paths in all actions
          ignoredActionsPaths: ['payload.privateKey', 'payload.schemas.definition'],
          // Ignore these paths in the state
          ignoredPaths: ['auth.privateKey', 'schemas.schemas.*.definition'],
        },
        immutableCheck: false
      })
  });
}

/**
 * Unified render function with Redux provider
 * @param {React.Component} ui - Component to render
 * @param {Object} options - Render options
 * @param {Object} options.preloadedState - Initial Redux state
 * @param {Object} options.store - Custom store instance
 * @param {Object} renderOptions - Additional render options
 * @returns {Object} Render result with store
 */
export function renderWithRedux(ui, {
  preloadedState = {},
  store = null,
  ...renderOptions
} = {}) {
  if (!store) {
    store = createTestStore(preloadedState);
  }
  
  function Wrapper({ children }) {
    return <Provider store={store}>{children}</Provider>;
  }

  return {
    ...render(ui, { wrapper: Wrapper, ...renderOptions }),
    store
  };
}

/**
 * Render hook with Redux provider
 * @param {Function} hook - Hook to render
 * @param {Object} options - Render options
 * @param {Object} options.preloadedState - Initial Redux state
 * @param {Object} options.store - Custom store instance
 * @param {Object} renderOptions - Additional render options
 * @returns {Object} Render result with store
 */
export function renderHookWithRedux(hook, {
  preloadedState = {},
  store = null,
  ...renderOptions
} = {}) {
  if (!store) {
    store = createTestStore(preloadedState);
  }
  
  function Wrapper({ children }) {
    return <Provider store={store}>{children}</Provider>;
  }

  return {
    ...renderHook(hook, { wrapper: Wrapper, ...renderOptions }),
    store
  };
}

/**
 * Create initial test state for schemas
 * @param {Object} overrides - State overrides
 * @returns {Object} Initial schemas state
 */
export function createTestSchemaState(overrides = {}) {
  const defaultState = {
    schemas: {
      schemas: {},  // Match actual store structure
      loading: {
        fetch: false,
        operations: {}
      },
      errors: {
        fetch: null,
        operations: {}
      },
      lastFetched: null,
      cache: {
        ttl: 300000,
        version: '2.1.0',
        lastUpdated: null
      },
      activeSchema: null
    }
  };
  
  // Deep merge the overrides
  if (overrides.schemas) {
    defaultState.schemas.schemas = { ...defaultState.schemas.schemas, ...overrides.schemas };
  }
  
  return defaultState;
}

/**
 * Creates mock schema data for testing
 * Uses declarative schema format matching the backend
 * @param {Object} overrides - Properties to override in mock schema
 * @returns {Object} Mock schema object
 */
export const createMockSchema = (overrides = {}) => {
  const defaults = {
    name: 'test_schema',
    state: SCHEMA_STATES.APPROVED,
    fields: ['id', 'name', 'created_at'],
    schema_type: { Single: {} }
  };
  
  // Merge overrides, converting old field format to array if needed
  const merged = { ...defaults, ...overrides };
  
  // Convert fields from object to array if provided in old format
  if (merged.fields && typeof merged.fields === 'object' && !Array.isArray(merged.fields)) {
    merged.fields = Object.keys(merged.fields);
  }
  
  return merged;
};

/**
 * Creates mock range schema data for testing
 * Uses declarative schema format matching the backend
 * @param {Object} overrides - Properties to override in mock range schema
 * @returns {Object} Mock range schema object
 */
export const createMockRangeSchema = (overrides = {}) => {
  const defaults = {
    name: 'test_range_schema',
    state: SCHEMA_STATES.APPROVED,
    fields: ['timestamp', 'value', 'metadata'],
    key: { range_field: 'timestamp' },
    schema_type: 'Range'
  };
  
  // Merge overrides, converting old field format to array if needed
  const merged = { ...defaults, ...overrides };
  
  // Convert fields from object to array if provided in old format
  if (merged.fields && typeof merged.fields === 'object' && !Array.isArray(merged.fields)) {
    merged.fields = Object.keys(merged.fields);
  }
  
  return merged;
};

/**
 * Creates a list of mock schemas with different states for testing
 * Uses declarative schema format matching the backend
 * @param {number} count - Number of schemas to create
 * @param {Object} baseProps - Base properties for all schemas
 * @returns {Array} Array of mock schema objects
 */
export const createMockSchemaList = (count = 3, baseProps = {}) => {
  const states = [SCHEMA_STATES.APPROVED, SCHEMA_STATES.AVAILABLE, SCHEMA_STATES.BLOCKED];
  
  return Array.from({ length: count }, (_, index) => {
    const schema = {
      name: `schema_${index}`,
      state: states[index % states.length],
      fields: ['id', 'data'],
      schema_type: { Single: {} },
      ...baseProps
    };
    
    // Convert fields from object to array if provided in baseProps
    if (schema.fields && typeof schema.fields === 'object' && !Array.isArray(schema.fields)) {
      schema.fields = Object.keys(schema.fields);
    }
    
    return schema;
  });
};

/**
 * Creates mock authentication state for testing
 * @param {Object} overrides - Properties to override in auth state
 * @returns {Object} Mock auth state object
 */
export const createMockAuthState = (overrides = {}) => ({
  isAuthenticated: true,
  privateKey: 'mock_private_key_' + Math.random().toString(36).substr(2, 9),
  systemKeyId: 'mock_system_key_id',
  publicKey: 'mock_public_key',
  loading: false,
  error: null,
  ...overrides
});

/**
 * Utility to wait for async operations with timeout
 * @param {Function} condition - Function that returns true when condition is met
 * @param {number} timeout - Maximum time to wait in milliseconds
 * @param {number} interval - Polling interval in milliseconds
 * @returns {Promise} Resolves when condition is met or rejects on timeout
 */
export const waitForCondition = async (
  condition,
  timeout = TEST_TIMEOUT_DEFAULT_MS,
  interval = MOCK_API_DELAY_MS
) => {
  const startTime = Date.now();
  
  while (Date.now() - startTime < timeout) {
    if (await condition()) {
      return;
    }
    await new Promise(resolve => setTimeout(resolve, interval));
  }
  
  throw new Error(`Condition not met within ${timeout}ms timeout`);
};

/**
 * Mock delay utility for simulating async operations
 * @param {number} ms - Delay in milliseconds
 * @returns {Promise} Promise that resolves after delay
 */
export const mockDelay = (ms = MOCK_API_DELAY_MS) => {
  return new Promise(resolve => setTimeout(resolve, ms));
};

/**
 * Creates a mock error object for testing error handling
 * @param {string} message - Error message
 * @param {number} status - HTTP status code
 * @param {Object} details - Additional error details
 * @returns {Error} Mock error object
 */
export const createMockError = (message = 'Test error', status = 500, details = {}) => {
  const error = new Error(message);
  error.status = status;
  error.details = details;
  error.toUserMessage = () => `User-friendly: ${message}`;
  return error;
};

/**
 * Validates test coverage against threshold
 * @param {Object} coverage - Coverage report object
 * @returns {boolean} True if coverage meets threshold
 */
export const validateCoverage = (coverage) => {
  const metrics = ['lines', 'functions', 'branches', 'statements'];
  
  return metrics.every(metric => {
    const percentage = coverage[metric]?.pct || 0;
    return percentage >= COVERAGE_THRESHOLD_PERCENT;
  });
};

/**
 * Creates a batch of test operations for integration testing
 * @param {Array} operations - Array of operation functions
 * @param {number} batchSize - Size of each batch
 * @returns {Array} Array of batched operations
 */
export const createTestBatch = (operations, batchSize = TEST_VALIDATION_BATCH_SIZE) => {
  const batches = [];
  
  for (let i = 0; i < operations.length; i += batchSize) {
    batches.push(operations.slice(i, i + batchSize));
  }
  
  return batches;
};

/**
 * Mock localStorage for testing
 */
export const mockLocalStorage = (() => {
  let store = {};
  
  return {
    getItem: (key) => store[key] || null,
    setItem: (key, value) => { store[key] = value.toString(); },
    removeItem: (key) => { delete store[key]; },
    clear: () => { store = {}; },
    get length() { return Object.keys(store).length; },
    key: (index) => Object.keys(store)[index] || null
  };
})();

/**
 * Mock sessionStorage for testing
 */
export const mockSessionStorage = (() => {
  let store = {};
  
  return {
    getItem: (key) => store[key] || null,
    setItem: (key, value) => { store[key] = value.toString(); },
    removeItem: (key) => { delete store[key]; },
    clear: () => { store = {}; },
    get length() { return Object.keys(store).length; },
    key: (index) => Object.keys(store)[index] || null
  };
})();

/**
 * Custom matcher for testing schema objects
 * @param {Object} received - Received schema object
 * @param {Object} expected - Expected schema properties
 * @returns {Object} Matcher result
 */
export const toBeValidSchema = (received, expected = {}) => {
  const requiredFields = ['name', 'state', 'fields'];
  const validStates = Object.values(SCHEMA_STATES);
  
  const missingFields = requiredFields.filter(field => !(field in received));
  
  if (missingFields.length > 0) {
    return {
      message: () => `Expected schema to have required fields: ${missingFields.join(', ')}`,
      pass: false
    };
  }
  
  if (!validStates.includes(received.state)) {
    return {
      message: () => `Expected schema state to be one of: ${validStates.join(', ')}, got: ${received.state}`,
      pass: false
    };
  }
  
  if (typeof received.fields !== 'object' || received.fields === null) {
    return {
      message: () => 'Expected schema to have valid fields object',
      pass: false
    };
  }
  
  // Check expected properties if provided
  for (const [key, value] of Object.entries(expected)) {
    if (received[key] !== value) {
      return {
        message: () => `Expected schema.${key} to be ${value}, got: ${received[key]}`,
        pass: false
      };
    }
  }
  
  return {
    message: () => 'Schema is valid',
    pass: true
  };
};

/**
 * Setup function for test environment
 * Call this in test setup files
 */
export const setupTestEnvironment = () => {
  // Mock localStorage and sessionStorage
  Object.defineProperty(window, 'localStorage', {
    value: mockLocalStorage,
    writable: true
  });
  
  Object.defineProperty(window, 'sessionStorage', {
    value: mockSessionStorage,
    writable: true
  });
  
  // Mock IntersectionObserver
  global.IntersectionObserver = class IntersectionObserver {
    constructor() {}
    disconnect() {}
    observe() {}
    unobserve() {}
  };
  
  // Mock ResizeObserver
  global.ResizeObserver = class ResizeObserver {
    constructor() {}
    disconnect() {}
    observe() {}
    unobserve() {}
  };
  
  // Mock matchMedia
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => {}
    })
  });
  
  // Add custom matchers
  expect.extend({
    toBeValidSchema
  });
};

/**
 * Cleanup function for test environment
 * Call this in test teardown
 */
export const cleanupTestEnvironment = () => {
  mockLocalStorage.clear();
  mockSessionStorage.clear();
  
  // Clear any timers
  if (typeof vi !== 'undefined' && vi.clearAllTimers) {
    vi.clearAllTimers();
  }
  if (typeof jest !== 'undefined' && jest.clearAllTimers) {
    jest.clearAllTimers();
  }
};

/**
 * Mock API responses for testing
 */
export const mockApiResponses = {
  schemas: {
    available: [
      { id: 'schema1', name: 'Test Schema 1', approved: false },
      { id: 'schema2', name: 'Test Schema 2', approved: false }
    ],
    approved: [
      { id: 'schema3', name: 'Approved Schema', approved: true }
    ]
  },
  fields: {
    schema1: [
      { name: 'field1', type: 'string' },
      { name: 'field2', type: 'number' }
    ]
  }
};

// Export all utilities as default
export default {
  createTestStore,
  renderWithRedux,
  renderHookWithRedux,
  createTestSchemaState,
  createMockSchema,
  createMockRangeSchema,
  createMockSchemaList,
  createMockAuthState,
  waitForCondition,
  mockDelay,
  createMockError,
  validateCoverage,
  createTestBatch,
  mockLocalStorage,
  mockSessionStorage,
  toBeValidSchema,
  setupTestEnvironment,
  cleanupTestEnvironment,
  mockApiResponses,
  SCHEMA_STATES
};