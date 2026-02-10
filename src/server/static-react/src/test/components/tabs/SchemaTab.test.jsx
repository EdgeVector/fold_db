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
    // Return a thunk function that returns a Promise
    const thunk = () => {
      const action = {
        type: actionType,
        payload: undefined,
        meta: {
          requestId: 'test-id',
          requestStatus: 'fulfilled'
        }
      }
      return Promise.resolve(action)
    }

    // Add fulfilled property to the thunk so it matches what createAsyncThunk returns
    thunk.fulfilled = { match: () => true }
    return thunk
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

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders approved schemas section', async () => {
    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })

    await waitFor(() => {
      expect(screen.getByText('No approved schemas found.')).toBeInTheDocument()
    })
  })

  it('displays approved schemas', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'schema1': { name: 'Schema1', state: 'Approved', fields: [] },
        'schema2': { name: 'Schema2', state: 'Approved', fields: [] },
        'schema3': { name: 'Schema3', state: 'Available', fields: [] }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })

    // Should display 2 approved schemas (schema1 and schema2)
    await waitFor(() => {
      expect(screen.getByText('Schema1')).toBeInTheDocument()
      expect(screen.getByText('Schema2')).toBeInTheDocument()
    })
  })

  it('displays no approved schemas message when empty', async () => {
    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })

    await waitFor(() => {
      expect(screen.getByText('No approved schemas found.')).toBeInTheDocument()
    })
  })

  it('shows block button for approved schemas', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: [] }
      }
    })

    await renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })

    await waitFor(() => {
      expect(screen.getByText('Block')).toBeInTheDocument()
    })
  })

  it('handles schema blocking', async () => {
    const { blockSchema } = await import('../../../store/schemaSlice')

    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: [] }
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
          schema_type: { Single: {} },
          fields: ['id', 'name', 'email']
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
      expect(screen.getByText('name')).toBeInTheDocument()
      expect(screen.getByText('email')).toBeInTheDocument()
    })
  })
})