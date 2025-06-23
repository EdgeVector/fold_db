import { describe, it, expect, beforeEach, vi } from 'vitest'
import { render } from '@testing-library/react'
import React from 'react'
import { signedRequest, signPayload } from '../../utils/authenticationWrapper'
import { AuthenticationProvider, getAuthContextInstance } from '../../auth/useAuth'
import { 
  setupAuthTestEnvironment, 
  createAuthenticatedStateMock,
  createUnauthenticatedStateMock,
  createSigningMocks 
} from '../utils/authMocks'

// Mock Ed25519 functions
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Mock the signing utilities
vi.mock('../../utils/signing', () => createSigningMocks())

// Mock the authentication context module
vi.mock('../../auth/useAuth', async () => {
  const actual = await vi.importActual('../../auth/useAuth')
  return {
    ...actual,
    getAuthContextInstance: vi.fn()
  }
})

// Test component for authentication context
function TestAuthProvider({ children }) {
  return (
    <AuthenticationProvider>
      <div>{children}</div>
    </AuthenticationProvider>
  )
}

describe('Authentication Wrapper Integration', () => {
  beforeEach(() => {
    setupAuthTestEnvironment()
  })

  describe('signedRequest() functionality', () => {
    it('throws error when authentication is missing', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createUnauthenticatedStateMock())

      const mockRequestFunction = vi.fn().mockResolvedValue({ success: true })

      await expect(signedRequest(mockRequestFunction)).rejects.toThrow(
        'Authentication required: This operation requires valid authentication'
      )

      expect(mockRequestFunction).not.toHaveBeenCalled()
    })

    it('executes request function when authenticated', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createAuthenticatedStateMock())

      const mockResponse = { success: true, data: 'test-data' }
      const mockRequestFunction = vi.fn().mockResolvedValue(mockResponse)

      const result = await signedRequest(mockRequestFunction)

      expect(result).toEqual(mockResponse)
      expect(mockRequestFunction).toHaveBeenCalledTimes(1)
    })

    it('propagates errors from request function', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createAuthenticatedStateMock())

      const testError = new Error('Request failed')
      const mockRequestFunction = vi.fn().mockRejectedValue(testError)

      await expect(signedRequest(mockRequestFunction)).rejects.toThrow('Request failed')
      expect(mockRequestFunction).toHaveBeenCalledTimes(1)
    })

    it('handles edge cases in authentication state', async () => {
      // Test null context
      vi.mocked(getAuthContextInstance).mockReturnValue(null)
      const mockRequestFunction = vi.fn().mockResolvedValue({ success: true })
      
      await expect(signedRequest(mockRequestFunction)).rejects.toThrow(
        'Authentication required: This operation requires valid authentication'
      )

      // Test partial authentication (missing private key)
      const partialAuth = {
        ...createAuthenticatedStateMock(),
        isAuthenticated: false,
        privateKey: null,
        publicKeyId: null
      }
      vi.mocked(getAuthContextInstance).mockReturnValue(partialAuth)
      
      await expect(signedRequest(mockRequestFunction)).rejects.toThrow(
        'Authentication required: This operation requires valid authentication'
      )
    })
  })

  describe('signPayload() functionality', () => {
    it('throws error when authentication is missing', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createUnauthenticatedStateMock())

      const testPayload = { action: 'test', data: 'example' }

      await expect(signPayload(testPayload)).rejects.toThrow(
        'Authentication required: This operation requires valid authentication'
      )
    })

    it('creates signed message when authenticated', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createAuthenticatedStateMock())

      const testPayload = { action: 'test', data: 'example' }
      const signedMessage = await signPayload(testPayload)

      expect(signedMessage).toHaveProperty('payload')
      expect(signedMessage).toHaveProperty('signature')
      expect(signedMessage).toHaveProperty('public_key_id', 'SYSTEM_WIDE_PUBLIC_KEY')
      expect(signedMessage).toHaveProperty('timestamp')
      expect(typeof signedMessage.timestamp).toBe('number')

      // Verify payload is correctly encoded
      const decodedPayload = JSON.parse(atob(signedMessage.payload))
      expect(decodedPayload).toEqual(testPayload)
    })
  })

  describe('Protected operation behavior', () => {
    it('allows unprotected operations without authentication', async () => {
      render(<TestAuthProvider />)

      // Simulate an unprotected API call that doesn't use signedRequest
      const unprotectedOperation = async () => {
        const response = await fetch('/api/public-endpoint')
        return response.json()
      }

      const result = await unprotectedOperation()
      expect(result).toBeDefined()
      expect(fetch).toHaveBeenCalledWith('/api/public-endpoint')
    })

    it('enforces authentication for protected operations', async () => {
      vi.mocked(getAuthContextInstance).mockReturnValue(createUnauthenticatedStateMock())

      const protectedOperation = async () => {
        return await signedRequest(async () => {
          const response = await fetch('/api/protected-endpoint')
          return response.json()
        })
      }

      await expect(protectedOperation()).rejects.toThrow('Authentication required')

      // When authenticated, the same operation should succeed
      vi.mocked(getAuthContextInstance).mockReturnValue(createAuthenticatedStateMock())
      
      const result = await protectedOperation()
      expect(result).toBeDefined()
    })
  })

  describe('Integration with authentication context', () => {
    it('works with authentication context from unified useAuth', async () => {
      const keyAuthState = createAuthenticatedStateMock()
      vi.mocked(getAuthContextInstance).mockReturnValue(keyAuthState)

      // Verify getAuthContextInstance returns the expected structure
      const authInstance = getAuthContextInstance()
      expect(authInstance).toHaveProperty('isAuthenticated', true)
      expect(authInstance).toHaveProperty('privateKey')
      expect(authInstance).toHaveProperty('publicKeyId')

      // Test that signedRequest works with this context
      const result = await signedRequest(async () => ({ test: 'success' }))
      expect(result).toEqual({ test: 'success' })
    })

    it('maintains backward compatibility for unsigned requests', async () => {
      render(<TestAuthProvider />)

      // Regular fetch calls should still work without signing
      const response = await fetch('/api/public-data')
      const result = await response.json()

      expect(fetch).toHaveBeenCalledWith('/api/public-data')
      expect(result).toBeDefined()
    })
  })
})