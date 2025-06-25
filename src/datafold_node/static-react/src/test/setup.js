import '@testing-library/jest-dom'
import { vi, beforeEach } from 'vitest'

// Make vi available globally as jest for compatibility
global.jest = vi

// Mock fetch globally for tests
global.fetch = vi.fn()

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

// Mock console methods to avoid noise in tests
global.console = {
  ...console,
  error: vi.fn(),
  warn: vi.fn(),
  log: vi.fn(),
}

// Reset all mocks before each test
beforeEach(() => {
  vi.clearAllMocks()
  fetch.mockClear()
  if (global.EventSource.mockClear) {
    global.EventSource.mockClear()
  }
  Element.prototype.scrollIntoView.mockClear()
})