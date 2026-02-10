/**
 * Test file for SchemaStatusBadge component
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen } from '@testing-library/react'
import SchemaStatusBadge from '../../../components/schema/SchemaStatusBadge'

describe('SchemaStatusBadge', () => {
  it('renders approved state correctly', () => {
    render(<SchemaStatusBadge state="approved" />)
    expect(screen.getByText('Approved')).toBeInTheDocument()
  })

  it('renders available state correctly', () => {
    render(<SchemaStatusBadge state="available" />)
    expect(screen.getByText('Available')).toBeInTheDocument()
  })

  it('renders blocked state correctly', () => {
    render(<SchemaStatusBadge state="blocked" />)
    expect(screen.getByText('Blocked')).toBeInTheDocument()
  })

  it('renders pending state correctly', () => {
    render(<SchemaStatusBadge state="pending" />)
    expect(screen.getByText('Pending')).toBeInTheDocument()
  })

  it('shows range schema indicator when isRangeSchema is true', () => {
    render(<SchemaStatusBadge state="approved" isRangeSchema={true} />)
    expect(screen.getByText('Range Key')).toBeInTheDocument()
  })

  it('does not show range schema indicator when isRangeSchema is false', () => {
    render(<SchemaStatusBadge state="approved" isRangeSchema={false} />)
    expect(screen.queryByText('Range Key')).not.toBeInTheDocument()
  })

  it('applies small size classes correctly', () => {
    render(<SchemaStatusBadge state="approved" size="sm" />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveClass('px-1.5', 'py-0.5', 'text-xs')
  })

  it('applies medium size classes correctly', () => {
    render(<SchemaStatusBadge state="approved" size="md" />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveClass('px-2.5', 'py-0.5', 'text-xs')
  })

  it('applies large size classes correctly', () => {
    render(<SchemaStatusBadge state="approved" size="lg" />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveClass('px-3', 'py-1', 'text-sm')
  })

  it('applies custom className', () => {
    render(<SchemaStatusBadge state="approved" className="custom-badge" />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveClass('custom-badge')
  })

  it('has proper accessibility attributes', () => {
    render(<SchemaStatusBadge state="approved" isRangeSchema={true} />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveAttribute('aria-label', 'Schema status: Approved, Range Schema')
  })

  it('shows tooltip when showTooltip is true', () => {
    render(<SchemaStatusBadge state="approved" showTooltip={true} />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveAttribute('title')
  })

  it('does not show tooltip when showTooltip is false', () => {
    render(<SchemaStatusBadge state="approved" showTooltip={false} />)
    const badge = screen.getByText('Approved')
    expect(badge).toHaveAttribute('title', '')
  })

  it('handles unknown state gracefully', () => {
    render(<SchemaStatusBadge state="unknown" />)
    expect(screen.getByText('Unknown')).toBeInTheDocument()
  })

  it('applies correct color classes for each state', () => {
    const { rerender } = render(<SchemaStatusBadge state="approved" />)
    let badge = screen.getByText('Approved')
    expect(badge).toHaveClass('badge', 'badge-success')

    rerender(<SchemaStatusBadge state="available" />)
    badge = screen.getByText('Available')
    expect(badge).toHaveClass('badge', 'badge-info')

    rerender(<SchemaStatusBadge state="blocked" />)
    badge = screen.getByText('Blocked')
    expect(badge).toHaveClass('badge', 'badge-error')

    rerender(<SchemaStatusBadge state="pending" />)
    badge = screen.getByText('Pending')
    expect(badge).toHaveClass('badge', 'badge-warning')
  })

  it('renders both badges when isRangeSchema is true', () => {
    render(<SchemaStatusBadge state="approved" isRangeSchema={true} />)
    expect(screen.getByText('Approved')).toBeInTheDocument()
    expect(screen.getByText('Range Key')).toBeInTheDocument()
  })
})