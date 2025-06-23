import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import App from '../App'
import { renderWithRedux, createAuthenticatedState, createUnauthenticatedState } from './utils/testHelpers'

// Mock fetch globally
global.fetch = vi.fn()

// Mock Ed25519 functions for authentication
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Mock API calls for authentication
vi.mock('../api/securityClient', () => ({
  getSystemPublicKey: vi.fn(() => Promise.resolve({
    success: true,
    key: {
      public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=',
      id: 'SYSTEM_WIDE_PUBLIC_KEY'
    }
  }))
}))

// Mock Redux thunks to prevent state override
vi.mock('../store/authSlice', async () => {
  const actual = await vi.importActual('../store/authSlice')
  return {
    ...actual,
    initializeSystemKey: vi.fn(() => ({ type: 'auth/initializeSystemKey/fulfilled', payload: {} })),
    validatePrivateKey: vi.fn(() => ({ type: 'auth/validatePrivateKey/fulfilled', payload: {} })),
    refreshSystemKey: vi.fn(() => ({ type: 'auth/refreshSystemKey/fulfilled', payload: {} }))
  }
})

// Mock the components to focus on App logic
vi.mock('../components/Header', () => ({
  default: () => <div data-testid="header">Header</div>
}))

vi.mock('../components/Footer', () => ({
  default: () => <div data-testid="footer">Footer</div>
}))

vi.mock('../components/StatusSection', () => ({
  default: () => <div data-testid="status-section">Status</div>
}))

vi.mock('../components/ResultsSection', () => ({
  default: ({ results }) => (
    <div data-testid="results-section">
      Results: {JSON.stringify(results)}
    </div>
  )
}))

vi.mock('../components/LogSidebar', () => ({
  default: () => <div data-testid="log-sidebar">Log Sidebar</div>
}))

vi.mock('../components/tabs/SchemaTab', () => ({
  default: ({ schemas, onResult, onSchemaUpdated }) => (
    <div data-testid="schema-tab">
      Schema Tab - {schemas.length} schemas
      <button onClick={() => onResult({ success: true })}>Trigger Result</button>
      <button onClick={() => onSchemaUpdated()}>Update Schema</button>
    </div>
  )
}))

vi.mock('../components/tabs/QueryTab', () => ({
  default: ({ schemas, onResult }) => (
    <div data-testid="query-tab">
      Query Tab - {schemas.length} schemas
      <button onClick={() => onResult({ query: 'test' })}>Execute Query</button>
    </div>
  )
}))

vi.mock('../components/tabs/MutationTab', () => ({
  default: ({ schemas, onResult }) => (
    <div data-testid="mutation-tab">
      Mutation Tab - {schemas.length} schemas
      <button onClick={() => onResult({ mutation: 'test' })}>Execute Mutation</button>
    </div>
  )
}))

vi.mock('../components/tabs/TransformsTab', () => ({
  default: ({ schemas, onResult }) => (
    <div data-testid="transforms-tab">
      Transforms Tab - {schemas.length} schemas
      <button onClick={() => onResult({ transform: 'test' })}>Execute Transform</button>
    </div>
  )
}))

vi.mock('../components/tabs/SchemaDependenciesTab', () => ({
  default: ({ schemas }) => (
    <div data-testid="dependencies-tab">
      Dependencies Tab - {schemas.length} schemas
    </div>
  )
}))

describe('App Component', () => {
  beforeEach(() => {
    // Mock successful API response for schemas
    fetch.mockImplementation((url) => {
      if (url === '/api/schemas') {
        return Promise.resolve({
          ok: true,
          json: async () => ({
            data: {
              'TestSchema1': 'Approved',
              'TestSchema2': 'Available'
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
              name: { field_type: 'string', writable: true }
            }
          })
        })
      }
      
      // Default fallback
      return Promise.resolve({
        ok: true,
        json: async () => ({ data: {} })
      })
    })
  })

  it('renders main layout components when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByTestId('header')).toBeInTheDocument()
    expect(screen.getByTestId('footer')).toBeInTheDocument()
    expect(screen.getByTestId('status-section')).toBeInTheDocument()
    expect(screen.getByTestId('log-sidebar')).toBeInTheDocument()
  })

  it('renders main layout components when unauthenticated', async () => {
    renderWithRedux(<App />, { initialState: createUnauthenticatedState() })
    
    expect(screen.getByTestId('header')).toBeInTheDocument()
    expect(screen.getByTestId('footer')).toBeInTheDocument()
    expect(screen.getByTestId('status-section')).toBeInTheDocument()
    expect(screen.getByTestId('log-sidebar')).toBeInTheDocument()
  })

  it('renders all navigation tabs', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Schemas')).toBeInTheDocument()
    expect(screen.getByText('Query')).toBeInTheDocument()
    expect(screen.getByText('Mutation')).toBeInTheDocument()
    expect(screen.getByText('Transforms')).toBeInTheDocument()
    expect(screen.getByText('Dependencies')).toBeInTheDocument()
    expect(screen.getByText('Keys')).toBeInTheDocument()
  })

  it('starts with keys tab active when unauthenticated', async () => {
    renderWithRedux(<App />, { initialState: createUnauthenticatedState() })
    
    // Should default to Keys tab when unauthenticated
    const keysButton = screen.getByText('Keys')
    expect(keysButton).toHaveClass('text-primary', 'border-b-2', 'border-primary')
  })

  it('allows switching to schemas tab when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Click schemas tab
    const schemasButton = screen.getByText('Schemas')
    fireEvent.click(schemasButton)
    
    await waitFor(() => {
      expect(screen.getByTestId('schema-tab')).toBeInTheDocument()
    })
    
    expect(schemasButton).toHaveClass('text-primary', 'border-b-2', 'border-primary')
  })

  it('fetches schemas on mount when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schemas')
    })
    
    // Switch to schemas tab to see results
    fireEvent.click(screen.getByText('Schemas'))
    
    await waitFor(() => {
      expect(screen.getByText('Schema Tab - 1 schemas')).toBeInTheDocument()
    })
  })

  it('handles schema fetch error gracefully when authenticated', async () => {
    fetch.mockRejectedValue(new Error('Network error'))
    
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schemas')
    })
    
    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    
    // Should still render with empty schemas
    await waitFor(() => {
      expect(screen.getByText('Schema Tab - 0 schemas')).toBeInTheDocument()
    })
  })

  it('switches tabs correctly when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Start with schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByTestId('schema-tab')).toBeInTheDocument()
    })
    
    // Click Query tab
    fireEvent.click(screen.getByText('Query'))
    expect(screen.getByTestId('query-tab')).toBeInTheDocument()
    expect(screen.queryByTestId('schema-tab')).not.toBeInTheDocument()
    
    // Click Mutation tab
    fireEvent.click(screen.getByText('Mutation'))
    expect(screen.getByTestId('mutation-tab')).toBeInTheDocument()
    expect(screen.queryByTestId('query-tab')).not.toBeInTheDocument()
    
    // Click Transforms tab
    fireEvent.click(screen.getByText('Transforms'))
    expect(screen.getByTestId('transforms-tab')).toBeInTheDocument()
    
    // Click Dependencies tab
    fireEvent.click(screen.getByText('Dependencies'))
    expect(screen.getByTestId('dependencies-tab')).toBeInTheDocument()
  })

  it('prevents tab switching when unauthenticated (AUTH-003)', async () => {
    renderWithRedux(<App />, { initialState: createUnauthenticatedState() })
    
    // Should start on Keys tab
    expect(screen.getByText('Key Management')).toBeInTheDocument()
    
    // Other tabs should be disabled
    const schemasTab = screen.getByText('Schemas')
    const queryTab = screen.getByText('Query')
    
    expect(schemasTab).toHaveAttribute('disabled')
    expect(queryTab).toHaveAttribute('disabled')
    
    // Clicking disabled tabs should not switch content
    fireEvent.click(schemasTab)
    expect(screen.queryByTestId('schema-tab')).not.toBeInTheDocument()
    expect(screen.getByText('Key Management')).toBeInTheDocument()
  })

  it('updates tab styling when switching (authenticated)', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    const queryButton = screen.getByText('Query')
    const schemasButton = screen.getByText('Schemas')
    const keysButton = screen.getByText('Keys')
    
    // Initially keys is active
    expect(keysButton).toHaveClass('text-primary')
    expect(schemasButton).toHaveClass('text-gray-500')
    
    // Click schemas tab
    fireEvent.click(schemasButton)
    
    expect(schemasButton).toHaveClass('text-primary')
    expect(keysButton).toHaveClass('text-gray-500')
    
    // Click query tab
    fireEvent.click(queryButton)
    
    expect(queryButton).toHaveClass('text-primary')
    expect(schemasButton).toHaveClass('text-gray-500')
  })

  it('clears results when switching tabs (authenticated)', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByTestId('schema-tab')).toBeInTheDocument()
    })
    
    // Trigger a result in schema tab
    fireEvent.click(screen.getByText('Trigger Result'))
    expect(screen.getByTestId('results-section')).toBeInTheDocument()
    
    // Switch to query tab
    fireEvent.click(screen.getByText('Query'))
    
    // Results should be cleared
    expect(screen.queryByTestId('results-section')).not.toBeInTheDocument()
  })

  it('displays results when operation completes (authenticated)', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByTestId('schema-tab')).toBeInTheDocument()
    })
    
    // Trigger a result
    fireEvent.click(screen.getByText('Trigger Result'))
    
    expect(screen.getByTestId('results-section')).toBeInTheDocument()
    expect(screen.getByText('Results: {"success":true}')).toBeInTheDocument()
  })

  it('refetches schemas when schema is updated (authenticated)', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByTestId('schema-tab')).toBeInTheDocument()
    })
    
    // Clear the initial fetch call
    fetch.mockClear()
    
    // Mock updated response
    fetch.mockResolvedValue({
      ok: true,
      json: async () => ({
        data: [
          { name: 'UpdatedSchema', fields: {} }
        ]
      })
    })
    
    // Trigger schema update
    fireEvent.click(screen.getByText('Update Schema'))
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schemas')
    })
  })

  it('passes correct props to tab components when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByText('Schema Tab - 1 schemas')).toBeInTheDocument()
    })
    
    // Switch to query tab
    fireEvent.click(screen.getByText('Query'))
    expect(screen.getByText('Query Tab - 1 schemas')).toBeInTheDocument()
    
    // Switch to mutation tab
    fireEvent.click(screen.getByText('Mutation'))
    expect(screen.getByText('Mutation Tab - 1 schemas')).toBeInTheDocument()
    
    // Switch to transforms tab
    fireEvent.click(screen.getByText('Transforms'))
    expect(screen.getByText('Transforms Tab - 1 schemas')).toBeInTheDocument()
    
    // Switch to dependencies tab
    fireEvent.click(screen.getByText('Dependencies'))
    expect(screen.getByText('Dependencies Tab - 1 schemas')).toBeInTheDocument()
  })

  it('handles results from different tabs when authenticated', async () => {
    renderWithRedux(<App />, { initialState: createAuthenticatedState() })
    
    // Test query tab result
    fireEvent.click(screen.getByText('Query'))
    await waitFor(() => {
      expect(screen.getByTestId('query-tab')).toBeInTheDocument()
    })
    
    fireEvent.click(screen.getByText('Execute Query'))
    expect(screen.getByText('Results: {"query":"test"}')).toBeInTheDocument()
    
    // Test mutation tab result
    fireEvent.click(screen.getByText('Mutation'))
    await waitFor(() => {
      expect(screen.getByTestId('mutation-tab')).toBeInTheDocument()
    })
    
    fireEvent.click(screen.getByText('Execute Mutation'))
    expect(screen.getByText('Results: {"mutation":"test"}')).toBeInTheDocument()
    
    // Test transforms tab result
    fireEvent.click(screen.getByText('Transforms'))
    await waitFor(() => {
      expect(screen.getByTestId('transforms-tab')).toBeInTheDocument()
    })
    
    fireEvent.click(screen.getByText('Execute Transform'))
    expect(screen.getByText('Results: {"transform":"test"}')).toBeInTheDocument()
  })

  it('filters schemas by approved state when authenticated', async () => {
    fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        data: {
          'ApprovedSchema': 'Approved'
        }
      })
    })

    renderWithRedux(<App />, { initialState: createAuthenticatedState() })

    // Switch to schemas tab
    fireEvent.click(screen.getByText('Schemas'))
    await waitFor(() => {
      expect(screen.getByText('Schema Tab - 1 schemas')).toBeInTheDocument()
    })
  })
})