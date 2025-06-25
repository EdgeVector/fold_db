/**
 * @fileoverview Testing Utilities for React Application
 * 
 * Provides comprehensive testing utilities for the React application including
 * Redux store setup, component rendering helpers, API mocking utilities,
 * and custom matchers for improved test development experience.
 * 
 * TASK-006: Testing Enhancement - Created testing utilities
 * 
 * @module testingUtilities
 * @since 2.0.0
 */

import { render } from '@testing-library/react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { schemaSlice } from '../../store/schemaSlice';
import { authSlice } from '../../store/authSlice';
import {
  TEST_TIMEOUT_MS,
  MOCK_DELAY_MS,
  COVERAGE_THRESHOLD_PERCENT,
  SCHEMA_STATES
} from '../../constants/schemas';

/**
 * Creates a test store with the same configuration as the production store
 * but with optional initial state for testing scenarios
 * 
 * @param {Object} initialState - Initial state for the store
 * @returns {Object} Configured Redux store for testing
 */
export const createTestStore = (initialState = {}) => {
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
      schemas: {},
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
        version: '2.0.0',
        lastUpdated: null
      },
      activeSchema: null
    }
  };

  return configureStore({
    reducer: {
      auth: authSlice.reducer,
      schemas: schemaSlice.reducer
    },
    preloadedState: {
      ...defaultState,
      ...initialState
    },
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({
        serializableCheck: {
          ignoredActions: ['persist/PERSIST']
        }
      })
  });
};

/**
 * Enhanced render function that provides Redux store and other providers
 * 
 * @param {React.ReactElement} ui - Component to render
 * @param {Object} options - Render options
 * @param {Object} options.initialState - Initial Redux state
 * @param {Object} options.store - Custom store instance
 * @param {Object} options.renderOptions - Additional render options
 * @returns {Object} Render result with store and utilities
 */
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

/**
 * Creates mock schema data for testing
 * 
 * @param {Object} overrides - Properties to override in mock schema
 * @returns {Object} Mock schema object
 */
export const createMockSchema = (overrides = {}) => ({
  name: 'test_schema',
  state: SCHEMA_STATES.APPROVED,
  fields: {
    id: { field_type: 'String' },
    name: { field_type: 'String' },
    created_at: { field_type: 'String' }
  },
  schema_type: 'Standard',
  ...overrides
});

/**
 * Creates mock range schema data for testing
 * 
 * @param {Object} overrides - Properties to override in mock range schema
 * @returns {Object} Mock range schema object
 */
export const createMockRangeSchema = (overrides = {}) => ({
  name: 'test_range_schema',
  state: SCHEMA_STATES.APPROVED,
  fields: {
    timestamp: { field_type: 'Range' },
    value: { field_type: 'Range' },
    metadata: { field_type: 'Range' }
  },
  schema_type: {
    Range: { range_key: 'timestamp' }
  },
  rangeInfo: {
    isRangeSchema: true,
    rangeField: {
      name: 'timestamp',
      type: 'Range'
    }
  },
  ...overrides
});

/**
 * Creates a list of mock schemas with different states for testing
 * 
 * @param {number} count - Number of schemas to create
 * @param {Object} baseProps - Base properties for all schemas
 * @returns {Array} Array of mock schema objects
 */
export const createMockSchemaList = (count = 3, baseProps = {}) => {
  const states = [SCHEMA_STATES.APPROVED, SCHEMA_STATES.AVAILABLE, SCHEMA_STATES.BLOCKED];
  
  return Array.from({ length: count }, (_, index) => ({
    name: `schema_${index}`,
    state: states[index % states.length],
    fields: {
      id: { field_type: 'String' },
      data: { field_type: index % 2 === 0 ? 'String' : 'Number' }
    },
    ...baseProps
  }));
};

/**
 * Creates mock authentication state for testing
 * 
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
 * 
 * @param {Function} condition - Function that returns true when condition is met
 * @param {number} timeout - Maximum time to wait in milliseconds
 * @param {number} interval - Polling interval in milliseconds
 * @returns {Promise} Resolves when condition is met or rejects on timeout
 */
export const waitForCondition = async (
  condition,
  timeout = TEST_TIMEOUT_MS,
  interval = MOCK_DELAY_MS
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
 * 
 * @param {number} ms - Delay in milliseconds
 * @returns {Promise} Promise that resolves after delay
 */
export const mockDelay = (ms = MOCK_DELAY_MS) => {
  return new Promise(resolve => setTimeout(resolve, ms));
};

/**
 * Creates a mock error object for testing error handling
 * 
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
 * 
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
 * 
 * @param {Array} operations - Array of operation functions
 * @param {number} batchSize - Size of each batch
 * @returns {Array} Array of batched operations
 */
export const createTestBatch = (operations, batchSize = 5) => {
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
 * 
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
  vi?.clearAllTimers?.();
  jest?.clearAllTimers?.();
};

// Export all utilities as default
export default {
  createTestStore,
  renderWithProviders,
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
  cleanupTestEnvironment
};