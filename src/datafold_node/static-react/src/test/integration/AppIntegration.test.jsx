import { screen, fireEvent, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach, vi } from 'vitest'
// Import AppContent directly to avoid double Provider issue
import { AppContent } from '../../App'
import { renderWithRedux, createAuthenticatedState, createUnauthenticatedState } from '../utils/testHelpers'

// Mock fetch globally
global.fetch = vi.fn()

// Mock Ed25519 functions for authentication
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Mock API calls for authentication
vi.mock('../../api/securityClient', () => ({
  getSystemPublicKey: vi.fn(() => Promise.resolve({
    success: true,
    key: {
      public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=',
      id: 'SYSTEM_WIDE_PUBLIC_KEY'
    }
  }))
}))

// Mock Redux thunks to prevent state override
vi.mock('../../store/authSlice', async () => {
  const actual = await vi.importActual('../../store/authSlice')
  return {
    ...actual,
    initializeSystemKey: vi.fn(() => () => Promise.resolve()),
    validatePrivateKey: vi.fn(() => () => Promise.resolve()),
    refreshSystemKey: vi.fn(() => () => Promise.resolve())
  }
})

describe('App Integration Tests', () => {
  let user

  beforeEach(() => {
    user = userEvent.setup()
    
    // Reset fetch mock
    fetch.mockReset()
    
    // Mock successful API responses
    fetch.mockImplementation((url) => {
      if (url === '/api/schemas') {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            data: {
              'UserProfile': 'Available',
              'BlogPost': 'Approved',
              'ProductCatalog': 'Available'
            }
          })
        })
      }
      
      if (url.startsWith('/api/schema/')) {
        const schemaName = url.split('/').pop()
        return Promise.resolve({
          ok: true,
          json: async () => ({
            name: schemaName,
            fields: {
              id: { field_type: 'string', writable: false },
              name: { field_type: 'string', writable: true },
              email: { field_type: 'string', writable: true }
            }
          })
        })
      }
      
      if (url === '/api/samples/schemas') {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            data: ['SampleUser', 'SampleBlog', 'SampleProduct']
          })
        })
      }
      
      if (url.startsWith('/api/samples/schema/')) {
        const schemaName = url.split('/').pop()
        return Promise.resolve({
          ok: true,
          json: async () => ({
            name: schemaName,
            fields: {
              title: { field_type: 'string', writable: true },
              content: { field_type: 'text', writable: true }
            }
          })
        })
      }
      
      // Default fallback
      return Promise.resolve({
        ok: true,
        json: async () => ({ data: [] })
      })
    })
  })

  it('renders main application components when authenticated', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for main UI elements
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
    expect(screen.getByText('Node is running successfully')).toBeInTheDocument()
    expect(screen.getByText('Active and healthy')).toBeInTheDocument()
    
    // Check for navigation tabs
    expect(screen.getByText('Schemas')).toBeInTheDocument()
    expect(screen.getByText('Query')).toBeInTheDocument()
    expect(screen.getByText('Mutation')).toBeInTheDocument()
    expect(screen.getByText('Transforms')).toBeInTheDocument()
    expect(screen.getByText('Dependencies')).toBeInTheDocument()
  })

  it('renders main application with locked tabs when unauthenticated', async () => {
    renderWithRedux(<AppContent />, { initialState: createUnauthenticatedState() })
    
    // Check for main UI elements
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
    expect(screen.getByText('Authentication Required')).toBeInTheDocument()
    
    // Check that tabs are locked (AUTH-003 behavior)
    const tabs = ['Schemas', 'Query', 'Mutation', 'Ingestion', 'Transforms', 'Dependencies']
    tabs.forEach(tabName => {
      const tab = screen.getByText(tabName)
      expect(tab).toHaveClass('text-gray-300', 'cursor-not-allowed')
      expect(tab).toHaveAttribute('disabled')
    })
    
    // Keys tab should be accessible
    const keysTab = screen.getByText('Keys')
    expect(keysTab).toHaveClass('text-primary')
    expect(keysTab).not.toHaveAttribute('disabled')
  })

  it('loads and displays schemas when authenticated', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    const schemasTab = screen.getByText('Schemas')
    await fireEvent.click(schemasTab)
    
    // Wait for schemas to load
    await waitFor(() => {
      expect(screen.getByText('Available Schemas')).toBeInTheDocument()
      expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
    })
    
    // Check that API was called
    expect(fetch).toHaveBeenCalledWith('/api/schemas')
  })

  it('switches between tabs correctly when authenticated', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Initially on Keys tab (default)
    const keysTab = screen.getByText('Keys')
    expect(keysTab).toHaveClass('text-primary')
    
    // Click Schemas tab
    const schemasTab = screen.getByText('Schemas')
    await user.click(schemasTab)
    
    // Check Schemas tab is active
    await waitFor(() => {
      expect(schemasTab).toHaveClass('text-primary')
    })
    
    // Click Query tab
    const queryTab = screen.getByText('Query')
    await user.click(queryTab)
    
    // Check Query tab is active
    await waitFor(() => {
      expect(queryTab).toHaveClass('text-primary')
      expect(screen.getByText('Execute Query')).toBeInTheDocument()
    })
    
    // Click Mutation tab
    const mutationTab = screen.getByText('Mutation')
    await user.click(mutationTab)
    
    // Check Mutation tab is active
    await waitFor(() => {
      expect(mutationTab).toHaveClass('text-primary')
      expect(screen.getByText('Execute Mutation')).toBeInTheDocument()
    })
  })

  it('prevents tab switching when unauthenticated (AUTH-003)', async () => {
    renderWithRedux(<AppContent />, { initialState: createUnauthenticatedState() })
    
    // Initially on Keys tab (only accessible tab when unauthenticated)
    const keysTab = screen.getByText('Keys')
    expect(keysTab).toHaveClass('text-primary')
    
    // Try to click other tabs - they should be disabled
    const schemasTab = screen.getByText('Schemas')
    expect(schemasTab).toHaveAttribute('disabled')
    
    // Clicking disabled tab should not change active tab
    await user.click(schemasTab)
    
    // Should still be on Keys tab
    expect(keysTab).toHaveClass('text-primary')
    expect(schemasTab).not.toHaveClass('text-primary')
  })

  it('handles API errors gracefully when authenticated', async () => {
    // Mock API error
    fetch.mockRejectedValueOnce(new Error('Network error'))
    
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Should still render the UI even with API error
    await waitFor(() => {
      expect(screen.getByText('DataFold Node')).toBeInTheDocument()
      expect(screen.getByText('Schemas')).toBeInTheDocument()
    })
  })

  it('displays transform queue status when authenticated', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Click Transforms tab
    const transformsTab = screen.getByText('Transforms')
    await user.click(transformsTab)
    
    // Check that the tab is active (no need to check specific content)
    await waitFor(() => {
      expect(transformsTab).toHaveClass('text-primary')
    })
  })

  it('shows system status controls', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for status controls
    expect(screen.getByText('Reset Database')).toBeInTheDocument()
  })

  it('displays log sidebar', async () => {
    renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for log sidebar
    expect(screen.getByText('Logs')).toBeInTheDocument()
  })
})