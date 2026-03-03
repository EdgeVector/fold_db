import React from 'react';
import { render } from '@testing-library/react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import authSlice from '../../store/authSlice';
import schemaSlice from '../../store/schemaSlice';
import ingestionSlice from '../../store/ingestionSlice';

/**
 * Test helper to render components with Redux store
 */
export const renderWithRedux = (
  ui,
  {
    preloadedState = {},
    initialState = {},
    store = configureStore({
      reducer: {
        auth: authSlice,
        schemas: schemaSlice,
        ingestion: ingestionSlice,
      },
      preloadedState: { ...preloadedState, ...initialState },
    }),
    ...renderOptions
  } = {}
) => {
  function Wrapper({ children }) {
    return <Provider store={store}>{children}</Provider>;
  }

  return { store, ...render(ui, { wrapper: Wrapper, ...renderOptions }) };
};

/**
 * Create a test store with reducers
 */
export const createTestStore = (preloadedState = {}) => {
  return configureStore({
    reducer: {
      auth: authSlice,
      schemas: schemaSlice,
      ingestion: ingestionSlice,
    },
    preloadedState,
  });
};

/**
 * Mock authenticated state for testing
 */
export const createAuthenticatedState = () => ({
  auth: {
    isAuthenticated: true,
    user: {
      id: 'test-user-id',
      username: 'testuser',
      role: 'admin',
    },
    loading: false,
    error: null,
  },
  schemas: {
    schemas: {
      'UserSchema': {
        name: 'UserSchema',
        state: 'approved',
        fields: {
          id: { field_type: 'String' },
          name: { field_type: 'String' },
          age: { field_type: 'Number' },
        },
      },
      'ProductSchema': {
        name: 'ProductSchema',
        state: 'approved',
        fields: {
          id: { field_type: 'String' },
          name: { field_type: 'String' },
          price: { field_type: 'Number' },
        },
      },
    },
    loading: false,
    error: null,
  },
});

/**
 * Mock unauthenticated state for testing
 */
export const createUnauthenticatedState = () => ({
  auth: {
    isAuthenticated: false,
    user: null,
    loading: false,
    error: null,
  },
  schemas: {
    schemas: {},
    loading: false,
    error: null,
  },
});

/**
 * Mock schemas state for testing
 */
export const mockSchemasState = {
  schema: {
    schemas: [
      {
        name: 'UserSchema',
        state: 'approved',
        fields: {
          id: { field_type: 'String' },
          name: { field_type: 'String' },
          age: { field_type: 'Number' },
          range_field: { field_type: 'Range' },
        },
      },
      {
        name: 'RangeTestSchema',
        state: 'approved',
        fields: {
          id: { field_type: 'String' },
          range_key: { field_type: 'Range' },
        },
      },
    ],
    loading: false,
    error: null,
  },
};

/**
 * Mock authenticated state for testing
 */
export const mockAuthenticatedState = {
  auth: {
    isAuthenticated: true,
    user: {
      id: 'test-user-id',
      username: 'testuser',
      role: 'admin',
    },
    loading: false,
    error: null,
  },
};

/**
 * Wait for element with timeout
 */
export const waitForElement = async (getElement, timeout = 5000) => {
  const start = Date.now();
  while (Date.now() - start < timeout) {
    try {
      const element = getElement();
      if (element) return element;
    } catch {
      // Element not found yet, continue waiting
    }
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`Element not found within ${timeout}ms`);
};

/**
 * Simulate user interaction delay
 */
export const userDelay = (ms = 100) => new Promise(resolve => setTimeout(resolve, ms));