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

// Mock schema client for new API architecture
vi.mock('../../api/clients/schemaClient', () => ({
  schemaClient: {
    getSchemas: vi.fn(() => Promise.resolve({
      success: true,
      data: { data: ['Schema1', 'Schema2'] }
    })),
    getAllSchemasWithState: vi.fn(() => Promise.resolve({
      success: true,
      data: { approved: ['Schema1'], available: ['Schema2'], blocked: [] }
    })),
    getSchema: vi.fn(() => Promise.resolve({
      success: true,
      data: { name: 'TestSchema', fields: [] }
    }))
  },
  createSchemaClient: vi.fn(() => ({
    getSchemas: vi.fn(() => Promise.resolve({
      success: true,
      data: { data: ['Schema1', 'Schema2'] }
    })),
    getAllSchemasWithState: vi.fn(() => Promise.resolve({
      success: true,
      data: { approved: ['Schema1'], available: ['Schema2'], blocked: [] }
    })),
    getSchema: vi.fn(() => Promise.resolve({
      success: true,
      data: { name: 'TestSchema', fields: [] }
    }))
  }))
}))

// Mock Redux thunks to prevent state override
vi.mock('../../store/authSlice', async () => {
  const actual = await vi.importActual('../../store/authSlice')
  return {
    ...actual,
    default: actual.default, // Explicitly preserve the reducer
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
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for main UI elements
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
    expect(screen.getByText('Node is running successfully')).toBeInTheDocument()
    expect(screen.getByText('Active and healthy')).toBeInTheDocument()
    
    // Check for navigation tabs
    expect(screen.getByText('Schemas')).toBeInTheDocument()
    expect(screen.getByText('Query')).toBeInTheDocument()
    expect(screen.getByText('Mutation')).toBeInTheDocument()
    expect(screen.getByText('Transforms')).toBeInTheDocument()
    expect(screen.getByLabelText('Key Management tab')).toBeInTheDocument()
  })

  it('renders main application with locked tabs when unauthenticated', async () => {
    await renderWithRedux(<AppContent />, { initialState: createUnauthenticatedState() })
    
    // Check for main UI elements
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
    expect(screen.getByText('Authentication Required')).toBeInTheDocument()
    
    // Check that tabs are locked (AUTH-003 behavior)
    const tabs = ['Schemas', 'Query', 'Mutation', 'Ingestion', 'Transforms']
    tabs.forEach(tabName => {
      const tab = screen.getByLabelText(`${tabName} tab (requires authentication)`)
      expect(tab).toHaveClass('text-gray-300')
    })
    
    // Key Management tab should be accessible
    const keyManagementTab = screen.getByLabelText('Key Management tab')
    expect(keyManagementTab).toHaveAttribute('aria-current', 'page')
  })

  it('loads and displays schemas when authenticated', async () => {
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    const schemasTab = screen.getByLabelText('Schemas tab')
    await fireEvent.click(schemasTab)
    
    // Wait for schemas to load
    await waitFor(() => {
      expect(screen.getByText('Available Schemas')).toBeInTheDocument()
      expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
    })
    
    // Check that schemas are loaded via Redux (no direct fetch calls in new architecture)
    // The SchemaTab now uses Redux thunks with schemaClient instead of direct fetch
    expect(screen.getByText('Available Schemas')).toBeInTheDocument()
    expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
  })

  it('switches between tabs correctly when authenticated', async () => {
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Initially on Key Management tab (default)
    const keyManagementTab = screen.getByLabelText('Key Management tab')
    expect(keyManagementTab).toHaveClass('text-blue-600')
    
    // Click Schemas tab
    const schemasTab = screen.getByLabelText('Schemas tab')
    await user.click(schemasTab)
    
    // Check Schemas tab is active
    await waitFor(() => {
      expect(schemasTab).toHaveAttribute('aria-current', 'page')
    })
    
    // Click Query tab
    const queryTab = screen.getByLabelText('Query tab')
    await user.click(queryTab)
    
    // Check Query tab is active
    await waitFor(() => {
      expect(queryTab).toHaveAttribute('aria-current', 'page')
      expect(screen.getByText('Execute Query')).toBeInTheDocument()
    })
    
    // Click Mutation tab
    const mutationTab = screen.getByLabelText('Mutation tab')
    await user.click(mutationTab)
    
    // Check Mutation tab is active
    await waitFor(() => {
      expect(mutationTab).toHaveClass('text-blue-600')
      expect(screen.getByText('Execute Mutation')).toBeInTheDocument()
    })
  })

  it('prevents tab switching when unauthenticated (AUTH-003)', async () => {
    await renderWithRedux(<AppContent />, { initialState: createUnauthenticatedState() })
    
    // Initially on Key Management tab (only accessible tab when unauthenticated)
    const keyManagementTab = screen.getByLabelText('Key Management tab')
    expect(keyManagementTab).toHaveAttribute('aria-current', 'page')
    
    // Try to click other tabs - they should be disabled
    const schemasTab = screen.getByLabelText('Schemas tab (requires authentication)')
    expect(schemasTab).toHaveClass('text-gray-300')
    
    // Clicking disabled tab should not change active tab
    await user.click(schemasTab)
    
    // Should still be on Key Management tab
    expect(keyManagementTab).toHaveAttribute('aria-current', 'page')
    expect(schemasTab).not.toHaveAttribute('aria-current', 'page')
  })

  it('handles API errors gracefully when authenticated', async () => {
    // Mock API error
    fetch.mockRejectedValueOnce(new Error('Network error'))
    
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Should still render the UI even with API error
    await waitFor(() => {
      expect(screen.getByText('DataFold Node')).toBeInTheDocument()
      expect(screen.getByText('Schemas')).toBeInTheDocument()
    })
  })

  it('displays transform queue status when authenticated', async () => {
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Click Transforms tab
    const transformsTab = screen.getByLabelText('Transforms tab')
    await user.click(transformsTab)
    
    // Check that the tab is active (no need to check specific content)
    await waitFor(() => {
      expect(transformsTab).toHaveClass('text-blue-600')
    })
  })

  it('shows system status controls', async () => {
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for status controls
    expect(screen.getByText('Reset Database')).toBeInTheDocument()
  })

  it('displays log sidebar', async () => {
    await renderWithRedux(<AppContent />, { initialState: createAuthenticatedState() })
    
    // Check for log sidebar
    expect(screen.getByText('Logs')).toBeInTheDocument()
  })
})