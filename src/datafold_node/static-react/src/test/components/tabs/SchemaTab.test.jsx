import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import SchemaTab from '../../../components/tabs/SchemaTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore'

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

  it('fetches sample schemas and all schemas on mount', async () => {
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
      expect(fetch).toHaveBeenCalledWith('/api/samples/schemas')
      expect(fetch).toHaveBeenCalledWith('/api/schemas')
    })
  })

  it('displays available schemas count', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'Schema1': 'Available',
            'Schema2': 'Available',
            'Schema3': 'Approved'
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
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
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'ApprovedSchema': 'Approved'
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('ApprovedSchema')).toBeInTheDocument()
    })
    
    expect(screen.getByText('Approved Schemas')).toBeInTheDocument()
  })

  it('shows approve and block buttons for available schemas', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'AvailableSchema': 'Available'
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
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
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'ApprovedSchema': 'Approved'
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    await waitFor(() => {
      expect(screen.getByText('Unload')).toBeInTheDocument()
    })
  })

  it('handles schema approval', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'TestSchema': 'Available'
          }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: {} })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
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

  it('handles schema unloading', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: {
            'ApprovedSchema': 'Approved'
          }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: {} })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })
    
    // Click unload button
    await waitFor(() => {
      const unloadButton = screen.getByText('Unload')
      fireEvent.click(unloadButton)
    })
    
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/schema/ApprovedSchema', { method: 'DELETE' })
    })
  })

  it('fetches and displays fields when expanding an approved schema', async () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: [] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          data: {
            ApprovedSchema: 'Approved'
          }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'ApprovedSchema',
          fields: {
            id: { field_type: 'string', writable: true }
          }
        })
      })

    renderWithRedux(<SchemaTab {...mockProps} />, {
      preloadedState: createTestSchemaState()
    })

    // Expand the approved schema to trigger field fetch
    await waitFor(() => {
      fireEvent.click(screen.getByText('ApprovedSchema'))
    })

    await waitFor(() => {
      expect(screen.getByText('id')).toBeInTheDocument()
    })
  })
})