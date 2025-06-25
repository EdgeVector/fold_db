/**
 * Test file for FieldWrapper component
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen } from '@testing-library/react'
import FieldWrapper from '../../../components/form/FieldWrapper'

describe('FieldWrapper', () => {
  const defaultProps = {
    label: 'Test Field',
    name: 'testField',
    children: <input type="text" id="field-testField" />
  }

  it('renders label correctly', () => {
    render(<FieldWrapper {...defaultProps} />)
    expect(screen.getByText('Test Field')).toBeInTheDocument()
  })

  it('shows required indicator when required is true', () => {
    render(<FieldWrapper {...defaultProps} required={true} />)
    expect(screen.getByLabelText('required')).toBeInTheDocument()
  })

  it('displays error message when error is provided', () => {
    const errorMessage = 'This field is required'
    render(<FieldWrapper {...defaultProps} error={errorMessage} />)
    expect(screen.getByRole('alert')).toHaveTextContent(errorMessage)
  })

  it('displays help text when provided and no error', () => {
    const helpText = 'Enter your name here'
    render(<FieldWrapper {...defaultProps} helpText={helpText} />)
    expect(screen.getByText(helpText)).toBeInTheDocument()
  })

  it('prioritizes error message over help text', () => {
    const errorMessage = 'This field is required'
    const helpText = 'Enter your name here'
    render(
      <FieldWrapper 
        {...defaultProps} 
        error={errorMessage} 
        helpText={helpText} 
      />
    )
    expect(screen.getByRole('alert')).toHaveTextContent(errorMessage)
    expect(screen.queryByText(helpText)).not.toBeInTheDocument()
  })

  it('renders children correctly', () => {
    render(<FieldWrapper {...defaultProps} />)
    expect(screen.getByRole('textbox')).toBeInTheDocument()
  })

  it('applies custom className', () => {
    const { container } = render(
      <FieldWrapper {...defaultProps} className="custom-class" />
    )
    expect(container.firstChild).toHaveClass('custom-class')
  })

  it('has proper accessibility attributes', () => {
    render(<FieldWrapper {...defaultProps} />)
    const label = screen.getByText('Test Field')
    expect(label).toHaveAttribute('for', 'field-testField')
  })
})