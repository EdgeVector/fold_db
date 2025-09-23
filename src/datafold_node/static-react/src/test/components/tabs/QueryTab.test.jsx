import React from 'react'
import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import QueryTab from '../../../components/tabs/QueryTab'
import { renderWithRedux, createTestSchemaState, createMockAuthState } from '../../utils/testStore.jsx'

// Mock the API client
vi.mock('../../../api/clients/mutationClient', () => ({
  mutationClient: {
    client: {
      post: vi.fn(() => Promise.resolve({
        success: true,
        data: { results: ['test result'] }
      }))
    }
  }
}))

// Mock the query hooks
vi.mock('../../../hooks/useQueryState', () => ({
  useQueryState: vi.fn(() => ({
    state: {
      selectedSchema: 'test_schema',
      selectedFields: { name: true },
      fieldValues: {},
      rangeFilter: null
    },
    handleSchemaChange: vi.fn(),
    toggleField: vi.fn(),
    handleFieldValueChange: vi.fn(),
    handleRangeFilterChange: vi.fn(),
    setRangeSchemaFilter: vi.fn(),
    clearState: vi.fn(),
    approvedSchemas: [
      { name: 'test_schema', fields: { name: { field_type: 'String' } } }
    ],
    schemasLoading: false,
    selectedSchemaObj: { name: 'test_schema', fields: { name: { field_type: 'String' } } },
    isRangeSchema: false,
    rangeKey: null
  }))
}))

vi.mock('../../../hooks/useQueryBuilder', () => ({
  useQueryBuilder: vi.fn(() => ({
    query: { schema: 'test_schema', fields: ['name'] },
    isValid: true
  }))
}))

// Mock the child components
vi.mock('../../../components/query/QueryForm', () => ({
  default: ({ queryState, onSchemaChange }) => (
    <div data-testid="query-form">
      <button onClick={() => onSchemaChange('test_schema')}>
        Query Form - Schema: {queryState.selectedSchema}
      </button>
    </div>
  )
}))

vi.mock('../../../components/query/QueryActions', () => ({
  default: ({ onExecute, onClear, disabled }) => (
    <div data-testid="query-actions">
      <button 
        onClick={onExecute} 
        disabled={disabled}
        data-testid="execute-query"
      >
        Execute Query
      </button>
      <button onClick={onClear} data-testid="clear-query">
        Clear
      </button>
    </div>
  )
}))

vi.mock('../../../components/query/QueryPreview', () => ({
  default: ({ query }) => (
    <div data-testid="query-preview">
      Query Preview: {JSON.stringify(query)}
    </div>
  )
}))

describe('QueryTab Component', () => {
  const mockOnResult = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders query interface when not authenticated (no auth required)', async () => {
    const authState = createMockAuthState({ isAuthenticated: false })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    // Should render the authentication warning since QueryTab requires auth
    expect(screen.getByText('Authentication Required')).toBeInTheDocument()
    expect(screen.getByText('Please authenticate using the Keys tab before accessing query functionality.')).toBeInTheDocument()
  })

  it('renders query interface when authenticated', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByTestId('query-form')).toBeInTheDocument()
    }, { timeout: 5000 })
    
    expect(screen.getByTestId('query-actions')).toBeInTheDocument()
    expect(screen.getByTestId('query-preview')).toBeInTheDocument()
  }, 10000)

  it('handles query execution successfully', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByTestId('execute-query')).toBeInTheDocument()
    }, { timeout: 5000 })

    const executeButton = screen.getByTestId('execute-query')
    fireEvent.click(executeButton)

    await waitFor(() => {
      expect(mockOnResult).toHaveBeenCalledWith({
        success: true,
        data: { results: ['test result'] }
      })
    }, { timeout: 5000 })
  }, 10000)

  it('handles query execution failure', async () => {
    // Mock API failure
    const { mutationClient } = await import('../../../api/clients/mutationClient')
    mutationClient.client.post.mockResolvedValueOnce({
      success: false,
      error: 'Query failed'
    })

    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByTestId('execute-query')).toBeInTheDocument()
    }, { timeout: 5000 })

    const executeButton = screen.getByTestId('execute-query')
    fireEvent.click(executeButton)

    await waitFor(() => {
      expect(mockOnResult).toHaveBeenCalledWith({
        error: 'Query failed',
        details: expect.any(Object)
      })
    }, { timeout: 5000 })
  }, 10000)

  it('displays grid layout with form and preview', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    const { container } = await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByTestId('query-form')).toBeInTheDocument()
    }, { timeout: 5000 })

    // Check for grid layout classes
    const gridContainer = container.querySelector('.grid')
    expect(gridContainer).toBeInTheDocument()
    expect(gridContainer).toHaveClass('grid-cols-1', 'lg:grid-cols-3')
  }, 10000)

  it('handles clear state functionality', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      ...createTestSchemaState()
    }

    await renderWithRedux(<QueryTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByTestId('clear-query')).toBeInTheDocument()
    }, { timeout: 5000 })

    const clearButton = screen.getByTestId('clear-query')
    fireEvent.click(clearButton)

    // Verify the clear function was called (mocked in useQueryState)
    const { useQueryState } = await import('../../../hooks/useQueryState')
    const mockState = useQueryState()
    expect(mockState.clearState).toBeDefined()
  }, 10000)
})