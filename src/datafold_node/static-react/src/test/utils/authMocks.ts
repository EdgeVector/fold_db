import { vi } from 'vitest'

/**
 * Common mock utilities for authentication-related tests
 * Consolidates repeated mock setup patterns across test files
 */

/**
 * Creates deterministic Ed25519 mocks for consistent testing
 * Returns the same values across all tests for predictable assertions
 */
export function createEd25519Mocks() {
  return {
    utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
    getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
    signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
  }
}

/**
 * Sets up the @noble/ed25519 mock with deterministic values
 * Call this in beforeEach or at the top of test files
 */
export function setupEd25519Mock() {
  vi.mock('@noble/ed25519', () => createEd25519Mocks())
}

/**
 * Creates a mock for the signing utilities with consistent signature format
 */
export function createSigningMocks() {
  return {
    createSignedMessage: vi.fn(async (payload, publicKeyId, _privateKey) => ({
      payload: btoa(JSON.stringify(payload)),
      signature: 'AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=',
      public_key_id: publicKeyId,
      timestamp: Math.floor(Date.now() / 1000)
    }))
  }
}


/**
 * Creates common fetch mock for security API endpoints
 */
export function createSecurityFetchMock() {
  return vi.fn((url, options) => {
    if (url === '/api/security/system-key') {
      if (options?.method === 'POST') {
        // Registration endpoint
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({
            success: true,
            public_key_id: 'test-key-id'
          })
        })
      } else {
        // GET endpoint - return existing system key
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({
            success: true,
            key: {
              public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=', // base64 of 32 bytes filled with 2
              id: 'SYSTEM_WIDE_PUBLIC_KEY'
            }
          })
        })
      }
    }
    return Promise.resolve({ ok: true, json: () => Promise.resolve({}) })
  })
}

/**
 * Common clipboard mock setup
 */
export function createClipboardMock() {
  const mockWriteText = vi.fn(() => Promise.resolve())
  
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText: mockWriteText },
    writable: true,
    configurable: true
  })
  
  return mockWriteText
}

/**
 * Sets up common test environment with all authentication mocks
 * Use this in beforeEach for comprehensive auth testing setup
 */
export function setupAuthTestEnvironment() {
  global.fetch = createSecurityFetchMock() as any
  vi.clearAllMocks()
  
  return {
    fetch: global.fetch
  }
}