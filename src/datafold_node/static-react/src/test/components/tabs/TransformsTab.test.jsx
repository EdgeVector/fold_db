import React from 'react'
import { screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import TransformsTab from '../../../components/tabs/TransformsTab'
import { renderWithRedux, createTestSchemaState, createMockAuthState } from '../../utils/testStore.jsx'

// Mock the transform client
vi.mock('../../../api/clients', () => ({
  transformClient: {
    getTransforms: vi.fn(),
    getQueue: vi.fn(),
    addToQueue: vi.fn(),
    refreshQueue: vi.fn()
  }
}))

describe('TransformsTab Component', () => {
  const mockOnResult = vi.fn()

  beforeEach(async () => {
    vi.clearAllMocks()
    const { transformClient } = await import('../../../api/clients')
    
    // Mock responses matching the actual transformClient interface
    transformClient.getTransforms.mockResolvedValue({
      data: {
        data: {},
        count: 0,
        timestamp: Date.now()
      },
      success: true,
      status: 200
    })
    transformClient.getQueue.mockResolvedValue({
      data: {
        queue: [],
        length: 0,
        isEmpty: true,
        processing: [],
        completed: [],
        failed: []
      },
      success: true,
      status: 200
    })
    transformClient.addToQueue.mockResolvedValue({
      data: {
        success: true,
        message: 'Transform added to queue',
        transformId: 'test-transform-123',
        queuePosition: 1
      },
      success: true,
      status: 200
    })
    transformClient.refreshQueue.mockResolvedValue({
      data: {
        queue: [],
        length: 0,
        isEmpty: true,
        processing: [],
        completed: [],
        failed: []
      },
      success: true,
      status: 200
    })
  })
it('renders transform viewer with basic elements', async () => {
  const authState = createMockAuthState({ isAuthenticated: true })
  const initialState = {
    auth: authState,
    schemas: {
      schemas: {}, // Empty schemas object
      loading: { fetch: false, operations: {} },
      errors: { fetch: null, operations: {} },
      lastFetch: null,
      cache: { ttl: 0, data: null }
    }
  }

  await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
    preloadedState: initialState
  })

  expect(screen.getByText('Transforms')).toBeInTheDocument()
  expect(screen.getByText(/Queue Status:/)).toBeInTheDocument()
  
  await waitFor(() => {
    expect(screen.getByText('Queue Status: Empty')).toBeInTheDocument()
  })
})

  it('displays "No transforms found" when schemas have no transforms', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'test_schema': {
            name: 'test_schema',
            state: 'Approved',
            fields: {
              id: { type: 'string', required: true }
            }
          }
        }, // Schemas with no transforms
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    expect(screen.getByText('No transforms found in schemas')).toBeInTheDocument()
  })

  it('displays transforms when schemas have transform fields', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'test_schema': {
            name: 'test_schema',
            state: 'Approved',
            fields: {
              id: { type: 'string', required: true },
              transformed_field: {
                type: 'string',
                transform: {
                  logic: 'UPPER(input)',
                  output: 'test_schema.transformed_field',
                  inputs: ['input']
                }
              }
            }
          }
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('test_schema')).toBeInTheDocument()
    })
    expect(screen.getByText('transformed_field')).toBeInTheDocument()
    expect(screen.getByText('UPPER(input)')).toBeInTheDocument()
    expect(screen.getByText('Add to Queue')).toBeInTheDocument()
  })

  it('handles adding transform to queue', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'test_schema': {
            name: 'test_schema',
            state: 'Approved',
            fields: {
              transformed_field: {
                type: 'string',
                transform: {
                  logic: 'UPPER(input)',
                  output: 'test_schema.transformed_field',
                  inputs: ['input']
                }
              }
            }
          }
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Add to Queue')).toBeInTheDocument()
    })

    const addButton = screen.getByText('Add to Queue')
    fireEvent.click(addButton)

    const { transformClient } = await import('../../../api/clients')
    await waitFor(() => {
      expect(transformClient.addToQueue).toHaveBeenCalledWith('test_schema.transformed_field')
    })
  })

  it('displays queue status with items when queue is not empty', async () => {
    const { transformClient } = await import('../../../api/clients')
    transformClient.getQueue.mockResolvedValue({
      data: {
        queue: ['schema1.field1', 'schema2.field2'],
        length: 2,
        isEmpty: false,
        processing: [],
        completed: [],
        failed: []
      },
      success: true,
      status: 200
    })

    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {},
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Queue Status: 2 transform(s) queued')).toBeInTheDocument()
    }, { timeout: 3000 })
  })

  it('displays transform queue section when queue has items', async () => {
    const { transformClient } = await import('../../../api/clients')
    transformClient.getQueue.mockResolvedValue({
      data: {
        queue: ['schema1.field1', 'schema2.field2'],
        length: 2,
        isEmpty: false,
        processing: [],
        completed: [],
        failed: []
      },
      success: true,
      status: 200
    })

    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {},
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Transform Queue')).toBeInTheDocument()
      expect(screen.getByText('schema1.field1')).toBeInTheDocument()
      expect(screen.getByText('schema2.field2')).toBeInTheDocument()
    })
  })

  it('shows loading state when adding to queue', async () => {
    const { transformClient } = await import('../../../api/clients')
    // Make addToQueue take some time to resolve
    transformClient.addToQueue.mockImplementation(() =>
      new Promise(resolve => setTimeout(() => resolve({
        data: {
          success: true,
          message: 'Transform added to queue',
          transformId: 'test-transform-123',
          queuePosition: 1
        },
        success: true,
        status: 200
      }), 100))
    )

    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'test_schema': {
            name: 'test_schema',
            state: 'Approved',
            fields: {
              transformed_field: {
                type: 'string',
                transform: {
                  logic: 'UPPER(input)',
                  output: 'test_schema.transformed_field',
                  inputs: ['input']
                }
              }
            }
          }
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Add to Queue')).toBeInTheDocument()
    })

    const addButton = screen.getByText('Add to Queue')
    fireEvent.click(addButton)

    expect(screen.getByText('Adding...')).toBeInTheDocument()

    await waitFor(() => {
      expect(screen.getByText('Add to Queue')).toBeInTheDocument()
    }, { timeout: 200 })
  })

  it('handles API errors when adding to queue', async () => {
    const { transformClient } = await import('../../../api/clients')
    transformClient.addToQueue.mockRejectedValue(new Error('API Error'))

    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'test_schema': {
            name: 'test_schema',
            state: 'Approved',
            fields: {
              transformed_field: {
                type: 'string',
                transform: {
                  logic: 'UPPER(input)',
                  output: 'test_schema.transformed_field',
                  inputs: ['input']
                }
              }
            }
          }
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Add to Queue')).toBeInTheDocument()
    })

    const addButton = screen.getByText('Add to Queue')
    fireEvent.click(addButton)

    await waitFor(() => {
      expect(screen.getByText('Error: API Error')).toBeInTheDocument()
    })
  })

  it('displays schema state badges correctly', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {
          'approved_schema': {
            name: 'approved_schema',
            state: 'Approved',
            fields: {
              field1: {
                type: 'string',
                transform: {
                  logic: 'UPPER(input)',
                  output: 'approved_schema.field1'
                }
              }
            }
          },
          'available_schema': {
            name: 'available_schema',
            state: 'Available',
            fields: {
              field2: {
                type: 'string',
                transform: {
                  logic: 'LOWER(input)',
                  output: 'available_schema.field2'
                }
              }
            }
          }
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    await waitFor(() => {
      expect(screen.getByText('Approved')).toBeInTheDocument()
      expect(screen.getByText('Available')).toBeInTheDocument()
    })
  })

  it('fetches and refreshes queue information periodically', async () => {
    const authState = createMockAuthState({ isAuthenticated: true })
    const initialState = {
      auth: authState,
      schemas: {
        schemas: {},
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetch: null,
        cache: { ttl: 0, data: null }
      }
    }

    await renderWithRedux(<TransformsTab onResult={mockOnResult} />, {
      preloadedState: initialState
    })

    const { transformClient } = await import('../../../api/clients')
    expect(transformClient.getQueue).toHaveBeenCalled()
    expect(transformClient.getTransforms).toHaveBeenCalled()
  })
})