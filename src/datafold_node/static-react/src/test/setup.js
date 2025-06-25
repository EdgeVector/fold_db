import '@testing-library/jest-dom'
import { vi, beforeEach } from 'vitest'
import { setupTestEnvironment, cleanupTestEnvironment } from './utils/testUtilities.jsx'
import { TEST_TIMEOUT_DEFAULT_MS } from './config/constants.js'

// Make vi available globally as jest for compatibility
global.jest = vi

// Setup test environment with mocks and matchers
setupTestEnvironment()

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