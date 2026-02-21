/**
 * Test file for TextField component
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import TextField from '../../../components/form/TextField'

describe('TextField', () => {
  const defaultProps = {
    name: 'testField',
    label: 'Test Field',
    value: '',
    onChange: jest.fn()
  }

  beforeEach(() => {
    jest.clearAllMocks()
  })

  it('renders with basic props', () => {
    render(<TextField {...defaultProps} />)
    expect(screen.getByLabelText('Test Field')).toBeInTheDocument()
    expect(screen.getByRole('textbox')).toBeInTheDocument()
  })

  it('displays value correctly', () => {
    render(<TextField {...defaultProps} value="test value" />)
    expect(screen.getByDisplayValue('test value')).toBeInTheDocument()
  })

  it('calls onChange when input changes', async () => {
    const user = userEvent.setup()
    render(<TextField {...defaultProps} />)
    const input = screen.getByRole('textbox')
    
    await user.type(input, 'hello')
    expect(defaultProps.onChange).toHaveBeenCalledWith('hello')
  })

  it('shows required indicator when required', () => {
    render(<TextField {...defaultProps} required />)
    expect(screen.getByLabelText('required')).toBeInTheDocument()
  })

  it('displays error state correctly', () => {
    render(<TextField {...defaultProps} error="This field is required" />)
    expect(screen.getByRole('alert')).toHaveTextContent('This field is required')
    expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true')
  })

  it('displays placeholder text', () => {
    render(<TextField {...defaultProps} placeholder="Enter text here" />)
    expect(screen.getByPlaceholderText('Enter text here')).toBeInTheDocument()
  })

  it('can be disabled', () => {
    render(<TextField {...defaultProps} disabled />)
    expect(screen.getByRole('textbox')).toBeDisabled()
  })

  it('supports different input types', () => {
    render(<TextField {...defaultProps} type="email" />)
    expect(screen.getByRole('textbox')).toHaveAttribute('type', 'email')
  })

  it('shows debouncing indicator when debounced', async () => {
    const user = userEvent.setup()
    render(<TextField {...defaultProps} debounced debounceMs={100} />)
    const input = screen.getByRole('textbox')
    
    await user.type(input, 'test')
    expect(screen.getByRole('status')).toBeInTheDocument() // Loading spinner
  })

  it('debounces onChange calls when debounced is true', async () => {
    const user = userEvent.setup()
    render(<TextField {...defaultProps} debounced debounceMs={100} />)
    const input = screen.getByRole('textbox')
    
    await user.type(input, 'test')
    
    // Should not call onChange immediately
    expect(defaultProps.onChange).not.toHaveBeenCalled()
    
    // Should call onChange after debounce delay
    await waitFor(() => {
      expect(defaultProps.onChange).toHaveBeenCalledWith('test')
    }, { timeout: 200 })
  })

  it('updates internal value when external value changes', () => {
    const { rerender } = render(<TextField {...defaultProps} value="initial" />)
    expect(screen.getByDisplayValue('initial')).toBeInTheDocument()
    
    rerender(<TextField {...defaultProps} value="updated" />)
    expect(screen.getByDisplayValue('updated')).toBeInTheDocument()
  })

  it('has proper accessibility attributes', () => {
    render(
      <TextField 
        {...defaultProps} 
        error="Error message"
        helpText="Help text"
      />
    )
    const input = screen.getByRole('textbox')
    expect(input).toHaveAttribute('aria-describedby')
    expect(input).toHaveAttribute('aria-invalid', 'true')
  })
})