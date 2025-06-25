/**
 * Test Store Utilities
 * TASK-010: Test Suite Fixes and Validation
 * 
 * Provides pre-configured Redux store instances for testing
 */

import { configureStore } from '@reduxjs/toolkit';
import { Provider } from 'react-redux';
import { render } from '@testing-library/react';
import schemaSlice from '../../store/slices/schemaSlice';

/**
 * Create a test store with optional initial state
 * @param {Object} preloadedState - Initial state for the store
 * @returns {Object} Configured test store
 */
export function createTestStore(preloadedState = {}) {
  return configureStore({
    reducer: {
      schemas: schemaSlice
    },
    preloadedState,
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({
        serializableCheck: false,
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
 * Create initial test state for schemas
 * @param {Object} overrides - State overrides
 * @returns {Object} Initial schemas state
 */
export function createTestSchemaState(overrides = {}) {
  return {
    schemas: {
      available: [],
      approved: [],
      blocked: [],
      fields: {},
      loading: {
        schemas: false,
        fields: false,
        approve: false,
        block: false,
        unload: false
      },
      errors: {
        fetch: null,
        approve: null,
        block: null,
        unload: null,
        fields: null
      },
      cache: {
        lastFetch: null,
        isValid: false
      },
      ...overrides
    }
  };
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
  createTestSchemaState,
  mockApiResponses
};