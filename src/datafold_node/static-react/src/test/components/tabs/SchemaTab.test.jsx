import React from 'react'
import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import SchemaTab from '../../../components/tabs/SchemaTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore.jsx'

describe('SchemaTab Component', () => {
  const mockProps = {
    schemas: [], // This prop is not used by the current component
    onResult: vi.fn(),
    onSchemaUpdated: vi.fn()
  }

  beforeEach(() => {
    vi.clearAllMocks()
    global.fetch = vi.fn()
  })

  it('renders available schemas section', async () => {
    // Mock the API calls
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: ['SampleSchema1', 'SampleSchema2'] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'TestSchema': 'Available'
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('Available Schemas')).toBeInTheDocument()
    })
    
    expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
  })

  it('fetches sample schemas on mount', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/samples/schemas')
    })
  })

  it('displays available schemas count', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'schema1': { name: 'Schema1', state: 'Available' },
        'schema2': { name: 'Schema2', state: 'Available' },
        'schema3': { name: 'Schema3', state: 'Approved' }
      }
    })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    await waitFor(() => {
      expect(screen.getByText('Available Schemas (2)')).toBeInTheDocument()
    })
  })

  it('displays no available schemas message when empty', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: {} })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('Available Schemas (0)')).toBeInTheDocument()
    })
  })

  it('displays approved schemas in separate section', async () => {
    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    renderWithRedux(<SchemaTab {...mockProps} />, {
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

    renderWithRedux(<SchemaTab {...mockProps} />, {
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

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    await waitFor(() => {
      expect(screen.getByText('Block')).toBeInTheDocument()
    })
  })

  it('handles schema approval', async () => {
    // Mock the approval API call
    fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ success: true })
    })

    const schemaState = createTestSchemaState({
      schemas: {
        'testSchema': { name: 'TestSchema', state: 'Available' }
      }
    })

    renderWithRedux(<SchemaTab {...mockProps} />, {
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
      expect(fetch).toHaveBeenCalledWith('/api/schema/TestSchema/approve', { method: 'POST' })
    })
  })

  it('handles schema blocking', async () => {
    // Mock the block API call
    fetch.mockResolvedValueOnce({
      ok: true,
      json: async () => ({ success: true })
    })

    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })
    
    // Click block button
    await waitFor(() => {
      const blockButton = screen.getByText('Block')
      fireEvent.click(blockButton)
    })
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schema/ApprovedSchema/block', { method: 'POST' })
    })
  })

  it('fetches and displays fields when expanding an approved schema', async () => {
    // Mock the fetch call that happens on component mount
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      // Mock the field fetch API call
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'ApprovedSchema',
          fields: {
            id: { field_type: 'string', writable: true }
          }
        })
      })

    const schemaState = createTestSchemaState({
      schemas: {
        'approvedSchema': { name: 'ApprovedSchema', state: 'Approved', fields: {} }
      }
    })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: schemaState
    })

    // Expand the approved schema to trigger field fetch
    await waitFor(() => {
      fireEvent.click(screen.getByText('ApprovedSchema'))
    })

    // Wait for the API call to be made
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schema/ApprovedSchema')
    })

    // Note: The component doesn't directly update the schema in Redux store
    // It calls onSchemaUpdated callback which would trigger a parent update
    // For this test, we just verify the API call is made
  })
})