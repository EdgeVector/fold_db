import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import MutationEditor from '../../components/tabs/mutation/MutationEditor'

describe('MutationEditor', () => {
  const mockFields = {
    event_name: {
      field_type: 'Single'
    },
    metrics_by_timeframe: {
      field_type: 'Range'
    },
    user_segments: {
      field_type: 'Range'
    }
  }

  const mockOnFieldChange = vi.fn()

  it('renders range fields with key-value interface', () => {
    render(
      <MutationEditor
        fields={mockFields}
        mutationType="Create"
        mutationData={{}}
        onFieldChange={mockOnFieldChange}
      />
    )

    // Check that range fields are rendered
    expect(screen.getByText('metrics_by_timeframe')).toBeInTheDocument()
    expect(screen.getByText('user_segments')).toBeInTheDocument()
    
    // Check for Range field type labels (should show "Range (Complex)" for non-range schemas)
    const rangeLabels = screen.getAllByText('Range (Complex)')
    expect(rangeLabels).toHaveLength(2)
  })

  it('allows adding key-value pairs to range fields', () => {
    render(
      <MutationEditor
        fields={mockFields}
        mutationType="Create"
        mutationData={{}}
        onFieldChange={mockOnFieldChange}
      />
    )

    // Find and click the "Add Key-Value Pair" buttons
    const addButtons = screen.getAllByText('Add Key-Value Pair')
    expect(addButtons.length).toBeGreaterThan(0)
    
    // Click the first add button
    fireEvent.click(addButtons[0])
    
    // Verify onFieldChange was called
    expect(mockOnFieldChange).toHaveBeenCalled()
  })

  it('displays existing key-value pairs', () => {
    const mutationDataWithRange = {
      metrics_by_timeframe: {
        '2024-01-01:daily': '500',
        '2024-01-01:hourly:09': '25'
      }
    }

    render(
      <MutationEditor
        fields={mockFields}
        mutationType="Create"
        mutationData={mutationDataWithRange}
        onFieldChange={mockOnFieldChange}
      />
    )

    // Check that existing key-value pairs are displayed
    expect(screen.getByDisplayValue('2024-01-01:daily')).toBeInTheDocument()
    expect(screen.getByDisplayValue('500')).toBeInTheDocument()
    expect(screen.getByDisplayValue('2024-01-01:hourly:09')).toBeInTheDocument()
    expect(screen.getByDisplayValue('25')).toBeInTheDocument()
  })

  it('allows removing key-value pairs', () => {
    const mutationDataWithRange = {
      metrics_by_timeframe: {
        '2024-01-01:daily': '500'
      }
    }

    render(
      <MutationEditor
        fields={mockFields}
        mutationType="Create"
        mutationData={mutationDataWithRange}
        onFieldChange={mockOnFieldChange}
      />
    )

    // Find and click the remove button (X icon)
    const removeButtons = screen.getAllByTitle('Remove this key-value pair')
    expect(removeButtons.length).toBeGreaterThan(0)
    
    fireEvent.click(removeButtons[0])
    
    // Verify onFieldChange was called to remove the pair
    expect(mockOnFieldChange).toHaveBeenCalled()
  })
})