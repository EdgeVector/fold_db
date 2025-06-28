// Mock Ed25519 functions for all tests (must be before any imports)
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Mock validatePrivateKey thunk to always dispatch a successful authentication
vi.mock('../../store/authSlice', async (importOriginal) => {
  const actual = await importOriginal();
  return {
    ...actual,
    validatePrivateKey: () => (dispatch) => {
      dispatch({ type: 'auth/validatePrivateKey/fulfilled', payload: { isAuthenticated: true } });
      return Promise.resolve();
    }
  };
});

// Now import everything else
import { render, screen, waitFor, act } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import React from 'react'
import { Provider } from 'react-redux'
import { createTestStore } from '../utils/testUtilities.jsx'
import authReducer, {
  initializeSystemKey,
  validatePrivateKey,
  refreshSystemKey,
  clearAuthentication
} from '../../store/authSlice'
import { useAppSelector, useAppDispatch } from '../../store/hooks'
import * as securityClient from '../../api/securityClient'

vi.spyOn(securityClient, 'getSystemPublicKey').mockImplementation(() => Promise.resolve({
  success: true,
  key: {
    public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=',
    id: 'SYSTEM_WIDE_PUBLIC_KEY'
  }
}));

// Test component that monitors Redux auth state
function ReduxAuthMonitor() {
  const dispatch = useAppDispatch()
  const authState = useAppSelector(state => state.auth)
  const { isAuthenticated, systemPublicKey, systemKeyId, isLoading, error } = authState
  
  // Track render count and state changes
  const [renderCount, setRenderCount] = React.useState(0)
  const [stateHistory, setStateHistory] = React.useState([])
  
  React.useEffect(() => {
    setRenderCount(prev => prev + 1)
    setStateHistory(prev => [...prev, {
      timestamp: Date.now(),
      isAuthenticated,
      hasSystemKey: !!systemPublicKey,
      isLoading
    }])
  }, [isAuthenticated, systemPublicKey, isLoading])
  
  return (
    <div>
      <div data-testid="render-count">{renderCount}</div>
      <div data-testid="is-authenticated">{isAuthenticated ? 'true' : 'false'}</div>
      <div data-testid="system-public-key">{systemPublicKey ?? ''}</div>
      <div data-testid="system-key-id">{systemKeyId ?? ''}</div>
      <div data-testid="is-loading">{isLoading ? 'true' : 'false'}</div>
      <div data-testid="error">{error ?? ''}</div>
      <div data-testid="state-history">{JSON.stringify(stateHistory)}</div>
      
      <button 
        data-testid="initialize-system-key"
        onClick={() => dispatch(initializeSystemKey())}
      >
        Initialize System Key
      </button>
      <button 
        data-testid="validate-private-key"
        onClick={() => dispatch(validatePrivateKey('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE='))}
      >
        Validate Private Key
      </button>
      <button 
        data-testid="refresh-system-key"
        onClick={() => dispatch(refreshSystemKey())}
      >
        Refresh System Key
      </button>
      <button 
        data-testid="clear-authentication"
        onClick={() => dispatch(clearAuthentication())}
      >
        Clear Authentication
      </button>
    </div>
  )
}

// Warning banner component (simulates App.jsx behavior)
function AuthWarningBanner() {
  const isAuthenticated = useAppSelector(state => state.auth.isAuthenticated)
  const [renderCount, setRenderCount] = React.useState(0)
  
  React.useEffect(() => {
    setRenderCount(prev => prev + 1)
  }, [isAuthenticated])
  
  return (
    <div>
      <div data-testid="warning-render-count">{renderCount}</div>
      <div data-testid="warning-visible">{!isAuthenticated ? 'visible' : 'hidden'}</div>
    </div>
  )
}

// Tab lock component (simulates App.jsx tab behavior)
function TabLockIndicator() {
  const isAuthenticated = useAppSelector(state => state.auth.isAuthenticated)
  const [renderCount, setRenderCount] = React.useState(0)
  
  React.useEffect(() => {
    setRenderCount(prev => prev + 1)
  }, [isAuthenticated])
  
  return (
    <div>
      <div data-testid="tab-render-count">{renderCount}</div>
      <div data-testid="tabs-locked">{!isAuthenticated ? 'locked' : 'unlocked'}</div>
    </div>
  )
}

function TestApp() {
  return (
    <div>
      <ReduxAuthMonitor />
      <AuthWarningBanner />
      <TabLockIndicator />
    </div>
  )
}

describe('Redux Authentication State Synchronization', () => {
  let store
  let user

  beforeEach(() => {
    user = userEvent.setup()
    // Create fresh store for each test using consolidated test store
    store = createTestStore()
  })

  it('AUTH-003 Test: Components re-render immediately when authentication state changes', async () => {
    render(
      <Provider store={store}>
        <TestApp />
      </Provider>
    )

    // Wait for initial render
    await waitFor(() => {
      expect(screen.getByTestId('render-count')).toHaveTextContent('1')
    })

    // Initial state verification
    expect(screen.getByTestId('is-authenticated')).toHaveTextContent('false')
    expect(screen.getByTestId('warning-visible')).toHaveTextContent('visible')
    expect(screen.getByTestId('tabs-locked')).toHaveTextContent('locked')

    // Initialize system key first
    await user.click(screen.getByTestId('initialize-system-key'))
    console.log('Redux state after initializeSystemKey:', store.getState())

    await waitFor(() => {
      expect(screen.getByTestId('system-public-key')).toHaveTextContent('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    })

    // Record render counts before authentication
    const initialRenderCount = parseInt(screen.getByTestId('render-count').textContent)
    const initialWarningRenderCount = parseInt(screen.getByTestId('warning-render-count').textContent)
    const initialTabRenderCount = parseInt(screen.getByTestId('tab-render-count').textContent)

    // Authenticate - this should trigger immediate state updates
    await user.click(screen.getByTestId('validate-private-key'))

    // AUTH-003 Critical Test: Verify immediate state propagation
    await waitFor(() => {
      // Authentication state should update immediately
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
    }, { timeout: 1000 })

    // WARNING BANNER should disappear immediately (AUTH-003 specific test)
    await waitFor(() => {
      expect(screen.getByTestId('warning-visible')).toHaveTextContent('hidden')
    }, { timeout: 100 }) // Very short timeout to catch synchronization issues

    // TABS should unlock immediately (AUTH-003 specific test)
    await waitFor(() => {
      expect(screen.getByTestId('tabs-locked')).toHaveTextContent('unlocked')
    }, { timeout: 100 }) // Very short timeout to catch synchronization issues

    // Verify all components re-rendered due to state change
    const finalRenderCount = parseInt(screen.getByTestId('render-count').textContent)
    const finalWarningRenderCount = parseInt(screen.getByTestId('warning-render-count').textContent)
    const finalTabRenderCount = parseInt(screen.getByTestId('tab-render-count').textContent)

    expect(finalRenderCount).toBeGreaterThan(initialRenderCount)
    expect(finalWarningRenderCount).toBeGreaterThan(initialWarningRenderCount)
    expect(finalTabRenderCount).toBeGreaterThan(initialTabRenderCount)
  })

  it('AUTH-003 Test: State synchronization timing across multiple components', async () => {
    render(
      <Provider store={store}>
        <TestApp />
      </Provider>
    )

    // Initialize system key
    await user.click(screen.getByTestId('initialize-system-key'))
    await waitFor(() => {
      expect(screen.getByTestId('system-public-key')).not.toHaveTextContent('')
    })

    // Track exact timing of state changes
    const _startTime = Date.now()
    
    // Authenticate
    await user.click(screen.getByTestId('validate-private-key'))

    // Check that all UI elements update within a very short time window
    const authUpdateTime = await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
      return Date.now()
    }, { timeout: 1000 })

    const warningUpdateTime = await waitFor(() => {
      expect(screen.getByTestId('warning-visible')).toHaveTextContent('hidden')
      return Date.now()
    }, { timeout: 100 })

    const tabUpdateTime = await waitFor(() => {
      expect(screen.getByTestId('tabs-locked')).toHaveTextContent('unlocked')
      return Date.now()
    }, { timeout: 100 })

    // AUTH-003 specific: All updates should happen within 50ms of each other
    const maxTimeDiff = Math.max(
      Math.abs(authUpdateTime - warningUpdateTime),
      Math.abs(authUpdateTime - tabUpdateTime),
      Math.abs(warningUpdateTime - tabUpdateTime)
    )


    // If this fails, it indicates AUTH-003 synchronization issues
    expect(maxTimeDiff).toBeLessThan(50) // 50ms tolerance for synchronization
  })

  it('AUTH-003 Test: Redux DevTools integration and meaningful action names', async () => {
    // Verify Redux DevTools configuration exists
    const storeConfig = store.getState()
    expect(storeConfig).toHaveProperty('auth')

    render(
      <Provider store={store}>
        <TestApp />
      </Provider>
    )

    // Test that Redux state is accessible for debugging
    let initialState = store.getState()
    expect(initialState.auth.isAuthenticated).toBe(false)

    // Initialize and authenticate
    await user.click(screen.getByTestId('initialize-system-key'))
    await waitFor(() => {
      expect(screen.getByTestId('system-public-key')).not.toHaveTextContent('')
    })

    let postInitState = store.getState()
    expect(postInitState.auth.systemPublicKey).not.toBe('')

    await user.click(screen.getByTestId('validate-private-key'))
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
    })

    let finalState = store.getState()
    expect(finalState.auth.isAuthenticated).toBe(true)


    // Verify DevTools can access meaningful state
    expect(finalState.auth).toHaveProperty('isAuthenticated', true)
    expect(finalState.auth).toHaveProperty('systemPublicKey')
    expect(finalState.auth).toHaveProperty('privateKey')
  })

  it('AUTH-003 Test: Race condition handling in concurrent authentication operations', async () => {
    render(
      <Provider store={store}>
        <TestApp />
      </Provider>
    )

    // Simulate rapid concurrent operations that could cause race conditions
    const operations = [
      () => store.dispatch(initializeSystemKey()),
      () => store.dispatch(refreshSystemKey()),
      () => store.dispatch(validatePrivateKey('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')),
    ]

    // Execute operations rapidly
    await act(async () => {
      const promises = operations.map(op => op())
      await Promise.allSettled(promises)
    })

    // Wait for final state to stabilize
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    }, { timeout: 2000 })

    // Verify we end up in a consistent state (no partial state corruption)
    const finalState = {
      isAuthenticated: screen.getByTestId('is-authenticated').textContent,
      systemKey: screen.getByTestId('system-public-key').textContent,
      error: screen.getByTestId('error').textContent
    }


    // Should not have error state after race conditions
    expect(finalState.error).toBe('')
    
    // Should have system key loaded
    expect(finalState.systemKey).not.toBe('')
  })

  it('AUTH-003 Test: Clear authentication immediately updates all UI components', async () => {
    render(
      <Provider store={store}>
        <TestApp />
      </Provider>
    )

    // Set up authenticated state
    await user.click(screen.getByTestId('initialize-system-key'))
    await waitFor(() => {
      expect(screen.getByTestId('system-public-key')).not.toHaveTextContent('')
    })

    await user.click(screen.getByTestId('validate-private-key'))
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
      expect(screen.getByTestId('warning-visible')).toHaveTextContent('hidden')
      expect(screen.getByTestId('tabs-locked')).toHaveTextContent('unlocked')
    })

    // Clear authentication
    await user.click(screen.getByTestId('clear-authentication'))

    // AUTH-003 Test: All UI should revert immediately
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('false')
    }, { timeout: 100 })

    await waitFor(() => {
      expect(screen.getByTestId('warning-visible')).toHaveTextContent('visible')
    }, { timeout: 100 })

    await waitFor(() => {
      expect(screen.getByTestId('tabs-locked')).toHaveTextContent('locked')
    }, { timeout: 100 })
  })
})