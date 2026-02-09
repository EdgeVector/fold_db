/**
 * Test file for TabNavigation component
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import TabNavigation from '../../components/TabNavigation.jsx'
import { DEFAULT_TABS } from '../../constants/ui.js'

describe('TabNavigation', () => {
  const defaultProps = {
    activeTab: 'ingestion',
    onTabChange: vi.fn()
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders all default tabs', () => {
    render(<TabNavigation {...defaultProps} />)

    // Check for tab labels as they appear in DEFAULT_TABS
    DEFAULT_TABS.forEach(tab => {
      expect(screen.getByRole('button', { name: `${tab.label} tab` })).toBeInTheDocument()
    })
  })

  it('renders main group tabs', () => {
    render(<TabNavigation {...defaultProps} />)

    // Main group tabs with their actual labels
    expect(screen.getByText('Smart Folder')).toBeInTheDocument()
    expect(screen.getByText('AI Query')).toBeInTheDocument()
    expect(screen.getByText('File Upload')).toBeInTheDocument()
  })

  it('renders advanced group tabs', () => {
    render(<TabNavigation {...defaultProps} />)

    // Advanced group tab
    expect(screen.getByText('Native Index')).toBeInTheDocument()
  })

  it('renders separator between main and advanced tabs', () => {
    const { container } = render(<TabNavigation {...defaultProps} />)

    // There should be a separator div between tab groups
    const nav = container.querySelector('nav')
    expect(nav).toBeInTheDocument()
  })

  it('highlights active tab correctly', () => {
    render(<TabNavigation {...defaultProps} activeTab="ingestion" />)

    const activeTab = screen.getByRole('button', { name: /ingestion tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
  })

  it('does not highlight inactive tabs', () => {
    render(<TabNavigation {...defaultProps} activeTab="ingestion" />)

    const inactiveTab = screen.getByRole('button', { name: /ai query tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })

  it('calls onTabChange when clicking a tab', () => {
    render(<TabNavigation {...defaultProps} />)

    const aiQueryTab = screen.getByRole('button', { name: /ai query tab/i })
    fireEvent.click(aiQueryTab)

    expect(defaultProps.onTabChange).toHaveBeenCalledWith('llm-query')
  })

  it('calls onTabChange with correct tab id', () => {
    const customTabs = [
      { id: 'custom', label: 'Custom', group: 'main' }
    ]

    render(<TabNavigation {...defaultProps} tabs={customTabs} />)

    const customTab = screen.getByRole('button', { name: /custom tab/i })
    fireEvent.click(customTab)

    expect(defaultProps.onTabChange).toHaveBeenCalledWith('custom')
  })

  it('renders custom tabs when provided', () => {
    const customTabs = [
      { id: 'custom1', label: 'Custom Tab 1', group: 'main' },
      { id: 'custom2', label: 'Custom Tab 2', group: 'main' }
    ]

    render(<TabNavigation {...defaultProps} tabs={customTabs} />)

    expect(screen.getByText('Custom Tab 1')).toBeInTheDocument()
    expect(screen.getByText('Custom Tab 2')).toBeInTheDocument()

    // Should not render default tabs when custom tabs provided
    expect(screen.queryByText('Ingestion')).not.toBeInTheDocument()
  })

  it('handles disabled tabs correctly', () => {
    const tabsWithDisabled = [
      { id: 'enabled', label: 'Enabled Tab', disabled: false, group: 'main' },
      { id: 'disabled', label: 'Disabled Tab', disabled: true, group: 'main' }
    ]

    render(<TabNavigation {...defaultProps} tabs={tabsWithDisabled} />)

    const enabledTab = screen.getByRole('button', { name: /enabled tab/i })
    const disabledTab = screen.getByRole('button', { name: /disabled tab/i })

    expect(enabledTab).toBeEnabled()
    expect(disabledTab).toBeDisabled()
  })

  it('applies custom className', () => {
    const { container } = render(
      <TabNavigation {...defaultProps} className="custom-nav" />
    )

    expect(container.firstChild).toHaveClass('custom-nav')
  })

  it('has proper accessibility attributes', () => {
    render(<TabNavigation {...defaultProps} activeTab="ingestion" />)

    const activeTab = screen.getByRole('button', { name: /json ingestion tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
    expect(activeTab).toHaveAttribute('aria-label', 'JSON Ingestion tab')

    const inactiveTab = screen.getByRole('button', { name: /ai query tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })

  it('renders as nav element', () => {
    const { container } = render(<TabNavigation {...defaultProps} />)

    expect(container.querySelector('nav')).toBeInTheDocument()
  })
})
