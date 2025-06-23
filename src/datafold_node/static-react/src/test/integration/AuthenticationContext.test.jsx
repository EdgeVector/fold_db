import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import React from 'react'
import { AuthenticationProvider, useAuth, getAuthContextInstance } from '../../auth/useAuth'
import { setupAuthTestEnvironment } from '../utils/authMocks'

// Mock Ed25519 functions
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Test component that uses the authentication context
function TestComponent() {
  const auth = useAuth()
  
  return (
    <div>
      <div data-testid="is-authenticated">{auth.isAuthenticated ? 'true' : 'false'}</div>
      <div data-testid="system-public-key">{auth.systemPublicKey || 'null'}</div>
      <div data-testid="system-key-id">{auth.systemKeyId || 'null'}</div>
      <div data-testid="has-private-key">{auth.privateKey ? 'true' : 'false'}</div>
      <div data-testid="public-key-id">{auth.publicKeyId || 'null'}</div>
      <div data-testid="is-loading">{auth.isLoading ? 'true' : 'false'}</div>
      <div data-testid="error">{auth.error || 'null'}</div>
      <button 
        data-testid="validate-key" 
        onClick={() => auth.validatePrivateKey('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')}
      >
        Validate Key
      </button>
      <button data-testid="clear-auth" onClick={auth.clearAuthentication}>
        Clear Auth
      </button>
      <button data-testid="refresh-system" onClick={auth.refreshSystemKey}>
        Refresh System Key
      </button>
    </div>
  )
}

describe('Global Authentication Context Integration', () => {
  let user

  beforeEach(() => {
    user = userEvent.setup()
    setupAuthTestEnvironment()
  })

  it('provides authentication state through context hook', async () => {
    render(
      <AuthenticationProvider>
        <TestComponent />
      </AuthenticationProvider>
    )

    // Initially should not be authenticated but should have system key
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('false')
      expect(screen.getByTestId('system-public-key')).toHaveTextContent('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
      expect(screen.getByTestId('system-key-id')).toHaveTextContent('SYSTEM_WIDE_PUBLIC_KEY')
      expect(screen.getByTestId('has-private-key')).toHaveTextContent('false')
      expect(screen.getByTestId('public-key-id')).toHaveTextContent('null')
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    })
  })

  it('updates authentication state when private key is validated', async () => {
    render(
      <AuthenticationProvider>
        <TestComponent />
      </AuthenticationProvider>
    )

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    })

    // Validate a private key
    await user.click(screen.getByTestId('validate-key'))

    // Should now be authenticated
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
      expect(screen.getByTestId('has-private-key')).toHaveTextContent('true')
      expect(screen.getByTestId('public-key-id')).toHaveTextContent('SYSTEM_WIDE_PUBLIC_KEY')
    })
  })

  it('clears authentication state when clearAuthentication is called', async () => {
    render(
      <AuthenticationProvider>
        <TestComponent />
      </AuthenticationProvider>
    )

    // Authenticate first
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    })
    await user.click(screen.getByTestId('validate-key'))
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
    })

    // Clear authentication
    await user.click(screen.getByTestId('clear-auth'))

    // Should no longer be authenticated
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('false')
      expect(screen.getByTestId('has-private-key')).toHaveTextContent('false')
      expect(screen.getByTestId('public-key-id')).toHaveTextContent('null')
    })
  })

  it('getAuthContextInstance provides non-hook access to same state', async () => {
    render(
      <AuthenticationProvider>
        <TestComponent />
      </AuthenticationProvider>
    )

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    })

    // Verify initial state through non-hook access
    const authInstance = getAuthContextInstance()
    expect(authInstance).not.toBeNull()
    expect(authInstance.isAuthenticated).toBe(false)
    expect(authInstance.systemPublicKey).toBe('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    expect(authInstance.systemKeyId).toBe('SYSTEM_WIDE_PUBLIC_KEY')

    // Authenticate
    await user.click(screen.getByTestId('validate-key'))
    await waitFor(() => {
      expect(screen.getByTestId('is-authenticated')).toHaveTextContent('true')
    })

    // Verify authenticated state through non-hook access
    const authenticatedInstance = getAuthContextInstance()
    expect(authenticatedInstance.isAuthenticated).toBe(true)
    expect(authenticatedInstance.privateKey).toBeInstanceOf(Uint8Array)
    expect(authenticatedInstance.publicKeyId).toBe('SYSTEM_WIDE_PUBLIC_KEY')
  })

  it('refreshSystemKey updates system key state', async () => {
    render(
      <AuthenticationProvider>
        <TestComponent />
      </AuthenticationProvider>
    )

    // Wait for initial load
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
    })

    // Refresh system key
    await user.click(screen.getByTestId('refresh-system'))

    // Should show loading state and then complete
    await waitFor(() => {
      expect(screen.getByTestId('is-loading')).toHaveTextContent('false')
      expect(screen.getByTestId('system-public-key')).toHaveTextContent('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    })

    // Verify fetch was called for refresh
    expect(fetch).toHaveBeenCalledWith('/api/security/system-key', expect.anything())
  })

  it('throws error when useAuth is used outside provider', () => {
    // Mock console.error to avoid test noise
    const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})
    
    expect(() => {
      render(<TestComponent />)
    }).toThrow('useAuth must be used within an AuthenticationProvider')
    
    consoleSpy.mockRestore()
  })

  it('handles authentication state persistence across components', async () => {
    function FirstComponent() {
      const auth = useAuth()
      return (
        <div>
          <div data-testid="first-auth-status">{auth.isAuthenticated ? 'authenticated' : 'not-authenticated'}</div>
          <button data-testid="first-validate" onClick={() => auth.validatePrivateKey('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')}>
            Validate
          </button>
        </div>
      )
    }

    function SecondComponent() {
      const auth = useAuth()
      return (
        <div>
          <div data-testid="second-auth-status">{auth.isAuthenticated ? 'authenticated' : 'not-authenticated'}</div>
          <div data-testid="second-has-key">{auth.privateKey ? 'has-key' : 'no-key'}</div>
        </div>
      )
    }

    render(
      <AuthenticationProvider>
        <FirstComponent />
        <SecondComponent />
      </AuthenticationProvider>
    )

    // Initially both should show not authenticated
    await waitFor(() => {
      expect(screen.getByTestId('first-auth-status')).toHaveTextContent('not-authenticated')
      expect(screen.getByTestId('second-auth-status')).toHaveTextContent('not-authenticated')
      expect(screen.getByTestId('second-has-key')).toHaveTextContent('no-key')
    })

    // Authenticate in first component
    await user.click(screen.getByTestId('first-validate'))

    // Both components should reflect authenticated state
    await waitFor(() => {
      expect(screen.getByTestId('first-auth-status')).toHaveTextContent('authenticated')
      expect(screen.getByTestId('second-auth-status')).toHaveTextContent('authenticated')
      expect(screen.getByTestId('second-has-key')).toHaveTextContent('has-key')
    })
  })
})