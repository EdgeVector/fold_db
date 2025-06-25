/**
 * Test file for TabNavigation component
 * Part of TASK-002: Component Extraction and Modularization
 */

import { render, screen, fireEvent } from '@testing-library/react'
import TabNavigation from '../../components/TabNavigation'
import { DEFAULT_TABS } from '../../constants/ui'

describe('TabNavigation', () => {
  const defaultProps = {
    activeTab: 'keys',
    isAuthenticated: false,
    onTabChange: jest.fn()
  }

  beforeEach(() => {
    jest.clearAllMocks()
  })

  it('renders all default tabs', () => {
    render(<TabNavigation {...defaultProps} />)
    
    DEFAULT_TABS.forEach(tab => {
      expect(screen.getByText(tab.label)).toBeInTheDocument()
    })
  })

  it('highlights active tab correctly', () => {
    render(<TabNavigation {...defaultProps} activeTab="schemas" />)
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
  })

  it('shows lock icon for auth-required tabs when not authenticated', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={false} />)
    
    const authRequiredTabs = DEFAULT_TABS.filter(tab => tab.requiresAuth)
    authRequiredTabs.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab.*authentication required`, 'i') })
      expect(tabButton).toBeInTheDocument()
    })
  })

  it('shows check mark for Keys tab when authenticated', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={true} />)
    
    const keysTab = screen.getByRole('button', { name: /keys tab.*authenticated/i })
    expect(keysTab).toBeInTheDocument()
  })

  it('disables auth-required tabs when not authenticated', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={false} />)
    
    const authRequiredTabs = DEFAULT_TABS.filter(tab => tab.requiresAuth)
    authRequiredTabs.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab`, 'i') })
      expect(tabButton).toBeDisabled()
    })
  })

  it('enables all tabs when authenticated', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={true} />)
    
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab`, 'i') })
      expect(tabButton).toBeEnabled()
    })
  })

  it('calls onTabChange when clicking enabled tab', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={true} />)
    
    const schemasTab = screen.getByRole('button', { name: /schemas tab/i })
    fireEvent.click(schemasTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('schemas')
  })

  it('does not call onTabChange when clicking disabled auth-required tab', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={false} />)
    
    const schemasTab = screen.getByRole('button', { name: /schemas tab/i })
    fireEvent.click(schemasTab)
    
    expect(defaultProps.onTabChange).not.toHaveBeenCalled()
  })

  it('allows clicking Keys tab when not authenticated', () => {
    render(<TabNavigation {...defaultProps} isAuthenticated={false} />)
    
    const keysTab = screen.getByRole('button', { name: /keys tab/i })
    fireEvent.click(keysTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('keys')
  })

  it('renders custom tabs when provided', () => {
    const customTabs = [
      { id: 'custom1', label: 'Custom Tab 1', requiresAuth: false },
      { id: 'custom2', label: 'Custom Tab 2', requiresAuth: true }
    ]
    
    render(<TabNavigation {...defaultProps} tabs={customTabs} />)
    
    expect(screen.getByText('Custom Tab 1')).toBeInTheDocument()
    expect(screen.getByText('Custom Tab 2')).toBeInTheDocument()
    
    // Should not render default tabs
    expect(screen.queryByText('Schemas')).not.toBeInTheDocument()
  })

  it('displays tab icons when provided', () => {
    const tabsWithIcons = [
      { id: 'test', label: 'Test Tab', requiresAuth: false, icon: '🧪' }
    ]
    
    render(<TabNavigation {...defaultProps} tabs={tabsWithIcons} />)
    
    expect(screen.getByText('🧪')).toBeInTheDocument()
  })

  it('handles disabled tabs correctly', () => {
    const tabsWithDisabled = [
      { id: 'enabled', label: 'Enabled Tab', requiresAuth: false, disabled: false },
      { id: 'disabled', label: 'Disabled Tab', requiresAuth: false, disabled: true }
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
    render(<TabNavigation {...defaultProps} activeTab="schemas" />)
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
    
    const inactiveTab = screen.getByRole('button', { name: /keys tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })
})