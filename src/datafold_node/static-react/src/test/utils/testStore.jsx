/**
 * Test Store Utilities
 * TASK-010: Test Suite Fixes and Validation
 * 
 * Provides pre-configured Redux store instances for testing
 */

import React from 'react';
import { configureStore } from '@reduxjs/toolkit';
import { Provider } from 'react-redux';
import { render, renderHook } from '@testing-library/react';
import schemaReducer from '../../store/schemaSlice';
import authReducer from '../../store/authSlice';

/**
 * Create a test store with optional initial state
 * @param {Object} preloadedState - Initial state for the store
 * @returns {Object} Configured test store
 */
export function createTestStore(preloadedState = {}) {
  return configureStore({
    reducer: {
      auth: authReducer,
      schemas: schemaReducer
    },
    preloadedState,
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
            'schemas/loadSchema/fulfilled'
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
 * Render component with Redux provider
 * @param {React.Component} ui - Component to render
 * @param {Object} options - Render options
 * @param {Object} options.preloadedState - Initial Redux state
 * @param {Object} options.store - Custom store instance
 * @param {Object} renderOptions - Additional render options
 * @returns {Object} Render result with store
 */
export function renderWithRedux(ui, {
  preloadedState = {},
  store = createTestStore(preloadedState),
  ...renderOptions
} = {}) {
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
  store = createTestStore(preloadedState),
  ...renderOptions
} = {}) {
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
      schemas: {},  // Match actual store structure - object indexed by schema ID
      loading: {
        fetch: false,
        operations: {}
      },
      errors: {
        fetch: null,
        operations: {}
      },
      lastFetched: null
    }
  };
  
  // Deep merge the overrides
  if (overrides.schemas) {
    defaultState.schemas.schemas = { ...defaultState.schemas.schemas, ...overrides.schemas };
  }
  
  return defaultState;
}

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

export default {
  createTestStore,
  renderWithRedux,
  renderHookWithRedux,
  createTestSchemaState,
  mockApiResponses
};