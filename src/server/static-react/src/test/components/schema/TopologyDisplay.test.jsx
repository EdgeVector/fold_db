import { describe, it, expect } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import TopologyDisplay from '../../../components/schema/TopologyDisplay'

describe('TopologyDisplay', () => {
  it('renders primitive string type', () => {
    const topology = {
      root: {
        type: 'Primitive',
        value: 'String'
      }
    }
    render(<TopologyDisplay topology={topology} />)
    expect(screen.getByText('string')).toBeInTheDocument()
  })

  it('renders primitive number type', () => {
    const topology = {
      root: {
        type: 'Primitive',
        value: 'Number'
      }
    }
    render(<TopologyDisplay topology={topology} />)
    expect(screen.getByText('number')).toBeInTheDocument()
  })

  it('renders any type', () => {
    const topology = {
      root: {
        type: 'Any'
      }
    }
    render(<TopologyDisplay topology={topology} />)
    expect(screen.getByText('any')).toBeInTheDocument()
  })

  it('renders array type', () => {
    const topology = {
      root: {
        type: 'Array',
        value: {
          type: 'Primitive',
          value: 'String'
        }
      }
    }
    render(<TopologyDisplay topology={topology} />)
    expect(screen.getByText(/Array</)).toBeInTheDocument()
    expect(screen.getByText('string')).toBeInTheDocument()
  })

  it('renders object type with collapsible fields', () => {
    const topology = {
      root: {
        type: 'Object',
        value: {
          name: {
            type: 'Primitive',
            value: 'String'
          },
          age: {
            type: 'Primitive',
            value: 'Number'
          }
        }
      }
    }
    const { container } = render(<TopologyDisplay topology={topology} />)
    
    // Root object is expanded by default (depth=0)
    // Should show field names immediately
    expect(screen.getByText('name')).toBeInTheDocument()
    expect(screen.getByText('age')).toBeInTheDocument()
    
    // Click to collapse
    const button = container.querySelector('button')
    fireEvent.click(button)
    
    // Should show collapsed state with field count
    expect(screen.getByText(/2 fields/)).toBeInTheDocument()
  })

  it('renders nested object type', () => {
    const topology = {
      root: {
        type: 'Object',
        value: {
          user: {
            type: 'Object',
            value: {
              id: {
                type: 'Primitive',
                value: 'Number'
              },
              name: {
                type: 'Primitive',
                value: 'String'
              }
            }
          }
        }
      }
    }
    const { container } = render(<TopologyDisplay topology={topology} />)
    
    // Root object is expanded by default (depth=0)
    // Should show user field immediately
    expect(screen.getByText('user')).toBeInTheDocument()
    
    // Nested object should be collapsed initially (depth=1)
    // Click on the nested user object to expand it
    const buttons = container.querySelectorAll('button')
    fireEvent.click(buttons[1]) // The nested user button
    
    // Should show nested fields after expanding
    expect(screen.getByText('id')).toBeInTheDocument()
    expect(screen.getByText('name')).toBeInTheDocument()
  })

  it('renders compact mode without border', () => {
    const topology = {
      root: {
        type: 'Primitive',
        value: 'String'
      }
    }
    const { container } = render(<TopologyDisplay topology={topology} compact={true} />)
    
    // Should not have the bg-gray-50 border container in compact mode
    expect(container.querySelector('.bg-gray-50')).not.toBeInTheDocument()
    expect(screen.getByText('string')).toBeInTheDocument()
  })

  it('renders message when topology is missing', () => {
    render(<TopologyDisplay topology={null} />)
    expect(screen.getByText('No topology defined')).toBeInTheDocument()
  })

  it('renders message when topology is undefined', () => {
    render(<TopologyDisplay />)
    expect(screen.getByText('No topology defined')).toBeInTheDocument()
  })
})

