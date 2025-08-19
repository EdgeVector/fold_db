import '@testing-library/jest-dom'
import { vi, beforeEach } from 'vitest'
import { setupTestEnvironment, cleanupTestEnvironment } from './utils/testUtilities.jsx'
import { setupMockServer } from './mocks/apiMocks.js'
import { TEST_TIMEOUT_DEFAULT_MS } from './config/constants.js'

// Make vi available globally as jest for compatibility
global.jest = vi

// Mock the production store to prevent Redux warnings during imports
vi.mock('../store/store', () => ({
  store: {
    getState: vi.fn(() => ({
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
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    })),
    dispatch: vi.fn(),
    subscribe: vi.fn(),
    replaceReducer: vi.fn()
  }
}))

// Setup WebCrypto API for tests
import { webcrypto } from 'node:crypto'
Object.defineProperty(globalThis, 'crypto', {
  value: webcrypto,
})

// Mock Response for MSW tests
global.Response = Response
global.TEST_TIMEOUT_MS = TEST_TIMEOUT_DEFAULT_MS

// Setup test environment with mocks and matchers
setupTestEnvironment()

// Setup MSW server for API mocking
setupMockServer()

// Mock EventSource for LogSidebar component
global.EventSource = vi.fn(() => ({
  onmessage: null,
  onerror: null,
  close: vi.fn(),
  addEventListener: vi.fn(),
  removeEventListener: vi.fn(),
}))

// Mock scrollIntoView for DOM elements
Element.prototype.scrollIntoView = vi.fn()

// Mock console methods to avoid noise in tests (but keep original for debugging)
const originalConsole = console
global.console = {
  ...console,
  error: vi.fn(),
  warn: vi.fn(),
  log: vi.fn(),
  debug: originalConsole.debug // Keep debug for test debugging
}

// Set default test timeout
vi.setConfig({ testTimeout: TEST_TIMEOUT_DEFAULT_MS })

// Reset all mocks before each test
beforeEach(() => {
  vi.clearAllMocks()
  cleanupTestEnvironment()
  
  if (fetch && typeof fetch.mockClear === 'function') {
    fetch.mockClear()
  }
  if (global.EventSource && typeof global.EventSource.mockClear === 'function') {
    global.EventSource.mockClear()
  }
  if (Element.prototype.scrollIntoView && typeof Element.prototype.scrollIntoView.mockClear === 'function') {
    Element.prototype.scrollIntoView.mockClear()
  }
})