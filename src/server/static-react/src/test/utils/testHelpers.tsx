import React from 'react'
import { render } from '@testing-library/react'
import { Provider } from 'react-redux'
// Import consolidated utilities from testUtilities.jsx
import { createTestStore } from './testUtilities.jsx'

// Re-export consolidated utilities
export { createTestStore }

// Common test states
export const createAuthenticatedState = () => ({
  auth: {
    isAuthenticated: true,
    systemPublicKey: 'mock-public-key-base64',
    systemKeyId: 'mock-key-id',
    privateKey: 'AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=', // Base64-encoded mock key
    publicKeyId: 'mock-public-key-id',
    isLoading: false,
    error: null,
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
      isValid: false,
      timestamp: null
    }
  },
})

export const createUnauthenticatedState = () => ({
  auth: {
    isAuthenticated: false,
    systemPublicKey: null,
    systemKeyId: null,
    privateKey: null,
    publicKeyId: null,
    isLoading: false,
    error: null,
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
      isValid: false,
      timestamp: null
    }
  },
})

// Enhanced render helper with proper Redux store setup
export const renderWithRedux = async (component: React.ReactElement, options: any = {}) => {
  const {
    initialState = {},
    store = null,
    ...renderOptions
  } = options

  const testStore = store || await createTestStore(initialState);

  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={testStore}>{children}</Provider>
  )

  return {
    ...render(component, { wrapper: Wrapper, ...renderOptions }),
    store: testStore,
  }
}

// Enhanced render helper that prevents thunk dispatch
export const renderWithReduxNoThunks = async (component: React.ReactElement, options: any = {}) => {
  const {
    initialState = {},
    store = null,
    ...renderOptions
  } = options

  const testStore = store || await createTestStore(initialState);

  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={testStore}>{children}</Provider>
  )

  return {
    ...render(component, { wrapper: Wrapper, ...renderOptions }),
    store: testStore,
  }
}