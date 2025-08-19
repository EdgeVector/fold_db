import React from 'react'
import { screen } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import SchemaDependenciesTab from '../../../components/tabs/SchemaDependenciesTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore.jsx'
import { selectAllSchemas } from '../../../store/schemaSlice'

// Mock the dependency utilities
const mockDependencyGraph = {
  nodes: ['schema_a', 'schema_b', 'schema_c'],
  edges: [
    { source: 'schema_a', target: 'schema_b', type: 'transform' },
    { source: 'schema_b', target: 'schema_c', type: 'reference' }
  ]
}

vi.mock('../../../utils/dependencyUtils', () => ({
  getDependencyGraph: vi.fn(() => mockDependencyGraph)
}))

describe('SchemaDependenciesTab Component', () => {
  beforeEach(async () => {
    vi.clearAllMocks()
    
    // Re-setup the mock to ensure it returns the expected data
    const { getDependencyGraph } = await import('../../../utils/dependencyUtils')
    getDependencyGraph.mockReturnValue(mockDependencyGraph)
  })

  it('renders empty dependency graph when no schemas exist', async () => {
    const { getDependencyGraph } = await import('../../../utils/dependencyUtils')
    getDependencyGraph.mockReturnValue({
      nodes: [],
      edges: []
    })

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {}, // Empty schemas object
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    // Should render the SVG container
    const svg = container.querySelector('svg')
    expect(svg).toBeInTheDocument()
    expect(svg).toHaveClass('w-full')

    // Should call getDependencyGraph with empty schemas array (result of selectAllSchemas)
    expect(getDependencyGraph).toHaveBeenCalledWith([])
  })

  it('renders dependency graph with schemas and their relationships', async () => {
    const mockSchemas = [
      {
        name: 'schema_a',
        fields: { id: { field_type: 'String' } }
      },
      {
        name: 'schema_b',
        fields: { ref_id: { field_type: 'String' } }
      },
      {
        name: 'schema_c',
        fields: { data: { field_type: 'String' } }
      }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2]
        },
        loading: {
          fetch: false,
          operations: {}
        },
        errors: {
          fetch: null,
          operations: {}
        },
        lastFetched: null,
        cache: {
          ttl: 300000,
          version: '2.1.0',
          lastUpdated: null
        },
        activeSchema: null
      }
    }

    const { container, store } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    const { getDependencyGraph } = await import('../../../utils/dependencyUtils')
    
    // Should call getDependencyGraph with the schemas array (result of selectAllSchemas)
    expect(getDependencyGraph).toHaveBeenCalledWith(mockSchemas)

    // Debug: Check what's actually in the DOM
    console.log('=== TEST DEBUG START ===')
    console.log('DOM content:', container.innerHTML)
    console.log('SVG elements found:', container.querySelector('svg'))
    console.log('Rect elements:', container.querySelectorAll('rect').length)
    console.log('=== TEST DEBUG END ===')

    // Should render SVG nodes for each schema
    expect(container.querySelectorAll('rect')).toHaveLength(3)
    expect(container.querySelectorAll('text')).toHaveLength(5) // 3 node labels + 2 edge labels
    
    // Should render schema names
    expect(screen.getByText('schema_a')).toBeInTheDocument()
    expect(screen.getByText('schema_b')).toBeInTheDocument()
    expect(screen.getByText('schema_c')).toBeInTheDocument()
  })

  it('renders edges between dependent schemas with correct types', async () => {
    const mockSchemas = [
      { name: 'schema_a', fields: {} },
      { name: 'schema_b', fields: {} },
      { name: 'schema_c', fields: {} }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2]
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    // Should render lines connecting schemas
    const lines = container.querySelectorAll('line')
    expect(lines).toHaveLength(2)

    // Should render edge type labels
    expect(screen.getByText('transform')).toBeInTheDocument()
    expect(screen.getByText('reference')).toBeInTheDocument()

    // Should have arrow markers
    const markers = container.querySelectorAll('marker')
    expect(markers).toHaveLength(1)
    expect(markers[0]).toHaveAttribute('id', 'arrow')
  })

  it('positions nodes vertically with correct spacing', async () => {
    const mockSchemas = [
      { name: 'schema_a', fields: {} },
      { name: 'schema_b', fields: {} },
      { name: 'schema_c', fields: {} }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2]
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    // Check node positioning through transform attributes
    const nodeGroups = container.querySelectorAll('g[transform]')
    expect(nodeGroups).toHaveLength(3)

    // First node should be at y=20 (calculated from component logic)
    expect(nodeGroups[0]).toHaveAttribute('transform', 'translate(100, 20)')
    // Second node should be at y=120 (20 + 100 spacing)
    expect(nodeGroups[1]).toHaveAttribute('transform', 'translate(100, 120)')
    // Third node should be at y=220 (20 + 200 spacing)
    expect(nodeGroups[2]).toHaveAttribute('transform', 'translate(100, 220)')
  })

  it('calculates correct SVG height based on number of nodes', async () => {
    const mockSchemas = [
      { name: 'schema_a', fields: {} },
      { name: 'schema_b', fields: {} },
      { name: 'schema_c', fields: {} }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2]
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    const svg = container.querySelector('svg')
    // Height = nodes.length * (nodeHeight + vSpacing) + 40
    // Height = 3 * (40 + 60) + 40 = 3 * 100 + 40 = 340
    expect(svg).toHaveAttribute('height', '340')
  })

  it('applies correct colors for different edge types', async () => {
    const mockSchemas = [
      { name: 'schema_a', fields: {} },
      { name: 'schema_b', fields: {} },
      { name: 'schema_c', fields: {} }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2]
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    const lines = container.querySelectorAll('line')
    
    // Transform edge should be blue (#2563eb)
    expect(lines[0]).toHaveAttribute('stroke', '#2563eb')
    
    // Reference edge should be green (#16a34a)
    expect(lines[1]).toHaveAttribute('stroke', '#16a34a')
  })

  it('renders nodes with correct styling', async () => {
    const mockSchema = { name: 'schema_a', fields: {} }

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchema
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    const rect = container.querySelector('rect')
    expect(rect).toHaveAttribute('width', '120')
    expect(rect).toHaveAttribute('height', '40')
    expect(rect).toHaveAttribute('rx', '4')
    expect(rect).toHaveAttribute('fill', '#f9fafb')
    expect(rect).toHaveAttribute('stroke', '#4b5563')
  })

  it('handles empty dependency graph gracefully', async () => {
    const { getDependencyGraph } = await import('../../../utils/dependencyUtils')
    getDependencyGraph.mockReturnValue({
      nodes: [],
      edges: []
    })

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {}, // Empty schemas object
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    const svg = container.querySelector('svg')
    expect(svg).toBeInTheDocument()
    
    // Should have minimal height (40px) for empty graph
    expect(svg).toHaveAttribute('height', '40')
    
    // Should not have any nodes or edges
    expect(container.querySelectorAll('rect')).toHaveLength(0)
    expect(container.querySelectorAll('line')).toHaveLength(0)
  })

  it('handles complex dependency graphs with multiple edge types', async () => {
    const { getDependencyGraph } = await import('../../../utils/dependencyUtils')
    getDependencyGraph.mockReturnValue({
      nodes: ['schema_a', 'schema_b', 'schema_c', 'schema_d'],
      edges: [
        { source: 'schema_a', target: 'schema_b', type: 'transform' },
        { source: 'schema_a', target: 'schema_c', type: 'reference' },
        { source: 'schema_b', target: 'schema_d', type: 'transform' },
        { source: 'schema_c', target: 'schema_d', type: 'reference' }
      ]
    })

    const mockSchemas = [
      { name: 'schema_a', fields: {} },
      { name: 'schema_b', fields: {} },
      { name: 'schema_c', fields: {} },
      { name: 'schema_d', fields: {} }
    ]

    const initialState = {
      auth: {
        isAuthenticated: false,
        privateKey: null,
        systemKeyId: null,
        publicKey: null,
        loading: false,
        error: null
      },
      schemas: {
        schemas: {
          schema_a: mockSchemas[0],
          schema_b: mockSchemas[1],
          schema_c: mockSchemas[2],
          schema_d: mockSchemas[3]
        },
        loading: { fetch: false, operations: {} },
        errors: { fetch: null, operations: {} },
        lastFetched: null,
        cache: { ttl: 300000, version: '2.1.0', lastUpdated: null },
        activeSchema: null
      }
    }

    const { container } = await renderWithRedux(<SchemaDependenciesTab />, {
      preloadedState: initialState
    })

    // Should render all nodes
    expect(container.querySelectorAll('rect')).toHaveLength(4)
    
    // Should render all edges
    expect(container.querySelectorAll('line')).toHaveLength(4)
    
    // Should render all schema names
    expect(screen.getByText('schema_a')).toBeInTheDocument()
    expect(screen.getByText('schema_b')).toBeInTheDocument()
    expect(screen.getByText('schema_c')).toBeInTheDocument()
    expect(screen.getByText('schema_d')).toBeInTheDocument()

    // Should render both edge types
    expect(screen.getAllByText('transform')).toHaveLength(2)
    expect(screen.getAllByText('reference')).toHaveLength(2)
  })
})