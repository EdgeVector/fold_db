import React from 'react'
import { screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import MutationTab from '../../../components/tabs/MutationTab'
import { renderWithRedux, createTestSchemaState } from '../../utils/testStore.jsx'

// Mock the API client
vi.mock('../../../api', () => ({
  MutationClient: vi.fn().mockImplementation(() => ({
    executeMutation: vi.fn(() => Promise.resolve({
      success: true,
      data: { mutationId: 'test-mutation-123' }
    })),
    executeQuery: vi.fn(() => Promise.resolve({
      success: true,
      data: []
    })),
    validateMutation: vi.fn(() => Promise.resolve({
      isValid: true
    }))
  })),
  createMutationClient: vi.fn(() => ({
    executeMutation: vi.fn(() => Promise.resolve({
      success: true,
      data: { mutationId: 'test-mutation-123' }
    }))
  }))
}))

// Mock crypto operations
vi.mock('../../../utils/cryptoUtils', () => ({
  signPayload: vi.fn(() => Promise.resolve('signed-payload-hash')),
  validateRangeKey: vi.fn(() => ({ isValid: true }))
}))

// Mock Redux hooks
const mockDispatch = vi.fn()
vi.mock('react-redux', async (importOriginal) => {
  const actual = await importOriginal()
  return {
    ...actual,
    useDispatch: () => mockDispatch
  }
})

describe('MutationTab Component', () => {
  const mockOnResult = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders mutation form with no schemas available', async () => {
    const schemaState = createTestSchemaState({ approvedSchemas: [] })
    
    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    expect(screen.getByText('No options available')).toBeInTheDocument()
    expect(screen.getByText('Operation Type')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Execute Mutation/i })).toBeInTheDocument()
  })

  it('renders mutation form with approved schemas', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' },
            age: { field_type: 'Integer' }
          }
        }
      ]
    })
    
    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    expect(screen.getByText('Operation Type')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Execute Mutation/i })).toBeInTheDocument()
  })

  it('displays disabled submit button when no schema selected', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' },
            age: { field_type: 'Integer' }
          }
        }
      ]
    })
    
    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    const submitButton = screen.getByRole('button', { name: /Execute Mutation/i })
    expect(submitButton).toBeDisabled()
  })

  it('renders form controls and structure', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' },
            age: { field_type: 'Integer' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    expect(screen.getByText('Schema')).toBeInTheDocument()
    expect(screen.getByText('Operation Type')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Execute Mutation/i })).toBeInTheDocument()
  })

  it('shows mutation type options', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    expect(screen.getByText('Insert')).toBeInTheDocument()
    expect(screen.getByText('Update')).toBeInTheDocument()
    expect(screen.getByText('Delete')).toBeInTheDocument()
  })

  it('handles basic form interactions', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    // Verify form renders
    const submitButton = screen.getByRole('button', { name: /Execute Mutation/i })
    expect(submitButton).toBeInTheDocument()
    expect(submitButton).toBeDisabled()
  })

  it('handles form submission attempt', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    // Just verify the form exists and can be interacted with
    const submitButton = screen.getByRole('button', { name: /Execute Mutation/i })
    expect(submitButton).toBeDisabled()
    
    // Verify operation type selector defaults to "Insert"
    const operationSelect = screen.getByDisplayValue('Insert')
    expect(operationSelect).toBeInTheDocument()
  })

  it('renders operation type selector correctly', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    const operationSelect = screen.getByLabelText('Operation Type')
    expect(operationSelect).toBeInTheDocument()
    
    // Test changing operation type
    fireEvent.change(operationSelect, { target: { value: 'Insert' } })
    expect(operationSelect.value).toBe('Insert')
  })

  it('validates component structure and basic functionality', async () => {
    const schemaState = createTestSchemaState({
      approvedSchemas: [
        {
          name: 'test_schema',
          fields: {
            name: { field_type: 'String' }
          }
        }
      ]
    })

    await renderWithRedux(<MutationTab onResult={mockOnResult} />, {
      preloadedState: schemaState
    })

    // Verify core elements exist
    expect(screen.getByText('Schema')).toBeInTheDocument()
    expect(screen.getByText('Operation Type')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: /Execute Mutation/i })).toBeInTheDocument()
    
    // Verify default state
    const submitButton = screen.getByRole('button', { name: /Execute Mutation/i })
    expect(submitButton).toBeDisabled()
  })
})