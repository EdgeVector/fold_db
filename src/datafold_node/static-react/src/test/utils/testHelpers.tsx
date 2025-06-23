import React from 'react'
import { render } from '@testing-library/react'
import { Provider } from 'react-redux'
import { configureStore } from '@reduxjs/toolkit'
import authReducer from '../../store/authSlice'

// Create a test store with optional initial state
export const createTestStore = (initialState = {}) => {
  return configureStore({
    reducer: {
      auth: authReducer,
    },
    preloadedState: initialState,
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({
        serializableCheck: {
          // Ignore these action types for testing
          ignoredActions: ['persist/PERSIST', 'persist/REHYDRATE'],
        },
      }),
  })
}

// Helper to render components with Redux Provider
export const renderWithRedux = (component: React.ReactElement, options: any = {}) => {
  const {
    initialState = {},
    store = createTestStore(initialState),
    ...renderOptions
  } = options

  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  )

  return {
    ...render(component, { wrapper: Wrapper, ...renderOptions }),
    store,
  }
}

// Common test states
export const createAuthenticatedState = () => ({
  auth: {
    isAuthenticated: true,
    systemPublicKey: 'mock-public-key-base64',
    systemKeyId: 'mock-key-id',
    privateKey: new Uint8Array(32).fill(1),
    publicKeyId: 'mock-public-key-id',
    isLoading: false,
    error: null,
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
})

// Helper to create a test store that doesn't dispatch thunks on mount
export const createTestStoreWithoutThunks = (initialState = {}) => {
  return configureStore({
    reducer: {
      auth: authReducer,
    },
    preloadedState: initialState,
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware({
        serializableCheck: {
          ignoredActions: ['persist/PERSIST', 'persist/REHYDRATE'],
          ignoredActionsPaths: ['payload.privateKey'],
          ignoredPaths: ['auth.privateKey'],
        },
      }),
  })
}

// Enhanced render helper that prevents thunk dispatch
export const renderWithReduxNoThunks = (component: React.ReactElement, options: any = {}) => {
  const {
    initialState = {},
    store = createTestStoreWithoutThunks(initialState),
    ...renderOptions
  } = options

  const Wrapper = ({ children }: { children: React.ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  )

  return {
    ...render(component, { wrapper: Wrapper, ...renderOptions }),
    store,
  }
}