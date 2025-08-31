import React from 'react'
import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import SchemaTab from '../../../components/tabs/SchemaTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore.jsx'

// Mock schemaClient
vi.mock('../../../api/clients/schemaClient', () => ({
  default: {
    getSchema: vi.fn(() => Promise.resolve({
      success: true,
      data: { name: 'test-schema', state: 'approved', fields: {} }
    }))
  }
}))

// Mock Redux actions
vi.mock('../../../store/schemaSlice', async () => {
  const actual = await vi.importActual('../../../store/schemaSlice')
  
  const createMockAction = (actionType) => vi.fn(() => {
    const action = {
      type: actionType,
      payload: undefined,
      meta: {
        requestId: 'test-id',
        requestStatus: 'fulfilled'
      }
    }
    // Add fulfilled matcher
    action.fulfilled = { match: () => true }
    return Promise.resolve(action)
  })
  
  return {
    ...actual,
    approveSchema: createMockAction('schemas/approveSchema/fulfilled'),
    blockSchema: createMockAction('schemas/blockSchema/fulfilled'),
    fetchSchemas: createMockAction('schemas/fetchSchemas/fulfilled')
  }
})

describe('SchemaTab Component', () => {
  const mockProps = {
    schemas: [], // This prop is not used by the current component
    onResult: vi.fn(),
    onSchemaUpdated: vi.fn()
  }

  let mockStore

  beforeEach(() => {
    vi.clearAllMocks()
    mockStore = {
      dispatch: vi.fn(),
      getState: vi.fn(() => ({
        schemas: {
          schemas: {},
          loading: { fetch: false, operations: {} },
          errors: { fetch: null, operations: {} }
        }
      })),
      subscribe: vi.fn()
    }
  })

  it('renders available schemas section', async () => {
    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('Available Schemas')).toBeInTheDocument()
    })
    
    expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
  })



  it('displays available schemas count', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'schema1': { name: 'Schema1', state: 'Available' },
        'schema2': { name: 'Schema2', state: 'Available' },
        'schema3': { name: 'Schema3', state: 'Approved' }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    await waitFor(() => {
      expect(screen.getByText('Available Schemas (2)')).toBeInTheDocument()
    })
  })

  it('displays no available schemas message when empty', async () => {
    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('No available schemas')).toBeInTheDocument()
    })
  })

  it('displays approved schemas in separate section', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    await waitFor(() => {
      expect(screen.getByText('ApprovedSchema')).toBeInTheDocument()
    })
    
    expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
  })

  it('shows approve and block buttons for available schemas', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'availableSchema': { name: 'AvailableSchema', state: 'Available' }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    // Expand the available schemas section
    await waitFor(() => {
      const summary = screen.getByText('Available Schemas (1)')
      fireEvent.click(summary)
    })
    
    await waitFor(() => {
      expect(screen.getByText('Approve')).toBeInTheDocument()
      // Available schemas should only have Approve button, not Block
      expect(screen.queryByText('Block')).not.toBeInTheDocument()
    })
  })

  it('shows unload button for approved schemas', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    await waitFor(() => {
      expect(screen.getByText('Block')).toBeInTheDocument()
    })
  })

  it('handles schema approval', async () => {
    const { approveSchema } = await import('../../../store/schemaSlice')

    const schemaState = createTestSchemaState({
      schemas: {
        'testSchema': { name: 'TestSchema', state: 'Available' }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    // Expand available schemas
    await waitFor(() => {
      const summary = screen.getByText('Available Schemas (1)')
      fireEvent.click(summary)
    })
    
    // Click approve button
    await waitFor(() => {
      const approveButton = screen.getByText('Approve')
      fireEvent.click(approveButton)
    })
    
    await waitFor(() => {
      expect(approveSchema).toHaveBeenCalledWith({ schemaName: 'TestSchema' })
    })
  })

  it('handles schema blocking', async () => {
    const { blockSchema } = await import('../../../store/schemaSlice')

    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    // Click block button
    await waitFor(() => {
      const blockButton = screen.getByText('Block')
      fireEvent.click(blockButton)
    })
    
    await waitFor(() => {
      expect(blockSchema).toHaveBeenCalledWith({ schemaName: 'ApprovedSchema' })
    })
  })

  it('fetches and displays fields when expanding an approved schema', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': {
          name: 'ApprovedSchema',
          state: 'Approved',
          fields: {
            id: { field_type: 'string', writable: true }
          }
        }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })

    // Expand the approved schema to display fields
    await waitFor(() => {
      fireEvent.click(screen.getByText('ApprovedSchema'))
    })

    // Verify fields are displayed
    await waitFor(() => {
      expect(screen.getByText('id')).toBeInTheDocument()
    })
  })
})