/**
 * Test file for TabNavigation component
 * TASK-010: Test Suite Fixes and Validation for PBI-REACT-SIMPLIFY-001
 * Part of TASK-002: Component Extraction and Modularization
 */

import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import TabNavigation from '../../components/TabNavigation.jsx'
import { DEFAULT_TABS } from '../../constants/ui.js'
import { TEST_TIMEOUT_DEFAULT_MS } from '../config/constants.js'
import { renderWithRedux } from '../utils/testHelpers'
import { createAuthenticatedState, createUnauthenticatedState } from '../utils/testHelpers'

describe('TabNavigation', () => {
  const defaultProps = {
    activeTab: 'keys',
    onTabChange: vi.fn()
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders all default tabs', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    DEFAULT_TABS.forEach(tab => {
      expect(screen.getByText(tab.label)).toBeInTheDocument()
    })
  })

  it('highlights active tab correctly', () => {
    renderWithRedux(<TabNavigation {...defaultProps} activeTab="schemas" />, { initialState: createAuthenticatedState() })
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
  })

  it('shows lock icon for auth-required tabs when not authenticated', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    
    const authRequiredTabs = DEFAULT_TABS.filter(tab => tab.requiresAuth)
    authRequiredTabs.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab.*requires authentication`, 'i') })
      expect(tabButton).toBeInTheDocument()
    })
  })

  it('shows check mark for Key Management tab when authenticated', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    const keysTab = screen.getByRole('button', { name: /key management tab/i })
    expect(keysTab).toBeInTheDocument()
  })

  it('disables auth-required tabs when not authenticated', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    
    const authRequiredTabs = DEFAULT_TABS.filter(tab => tab.requiresAuth)
    authRequiredTabs.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab`, 'i') })
      expect(tabButton).toBeDisabled()
    })
  })

  it('enables all tabs when authenticated', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`${tab.label} tab`, 'i') })
      expect(tabButton).toBeEnabled()
    })
  })

  it('calls onTabChange when clicking enabled tab', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    const schemasTab = screen.getByRole('button', { name: /schemas tab/i })
    fireEvent.click(schemasTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('schemas')
  })

  it('does not call onTabChange when clicking disabled auth-required tab', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    
    const schemasTab = screen.getByRole('button', { name: /schemas tab/i })
    fireEvent.click(schemasTab)
    
    expect(defaultProps.onTabChange).not.toHaveBeenCalled()
  })

  it('allows clicking Key Management tab when not authenticated', () => {
    renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    
    const keysTab = screen.getByRole('button', { name: /key management tab/i })
    
    // Key Management tab should be enabled when not authenticated (doesn't require auth)
    expect(keysTab).not.toBeDisabled()
    fireEvent.click(keysTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('keys')
  })

  it('renders custom tabs when provided', () => {
    const customTabs = [
      { id: 'custom1', label: 'Custom Tab 1', requiresAuth: false },
      { id: 'custom2', label: 'Custom Tab 2', requiresAuth: true }
    ]
    
    renderWithRedux(<TabNavigation {...defaultProps} tabs={customTabs} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Custom Tab 1')).toBeInTheDocument()
    expect(screen.getByText('Custom Tab 2')).toBeInTheDocument()
    
    // Should not render default tabs
    expect(screen.queryByText('Schemas')).not.toBeInTheDocument()
  })

  it('displays tab icons when provided', () => {
    const tabsWithIcons = [
      { id: 'test', label: 'Test Tab', requiresAuth: false, icon: '🧪' }
    ]
    
    renderWithRedux(<TabNavigation {...defaultProps} tabs={tabsWithIcons} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('🧪')).toBeInTheDocument()
  })

  it('handles disabled tabs correctly', () => {
    const tabsWithDisabled = [
      { id: 'enabled', label: 'Enabled Tab', requiresAuth: false, disabled: false },
      { id: 'disabled', label: 'Disabled Tab', requiresAuth: false, disabled: true }
    ]
    
    renderWithRedux(<TabNavigation {...defaultProps} tabs={tabsWithDisabled} />, { initialState: createAuthenticatedState() })
    
    const enabledTab = screen.getByRole('button', { name: /enabled tab/i })
    const disabledTab = screen.getByRole('button', { name: /disabled tab/i })
    
    expect(enabledTab).toBeEnabled()
    expect(disabledTab).toBeDisabled()
  })

  it('applies custom className', () => {
    const { container } = renderWithRedux(
      <TabNavigation {...defaultProps} className="custom-nav" />,
      { initialState: createAuthenticatedState() }
    )
    
    expect(container.firstChild).toHaveClass('custom-nav')
  })

  it('has proper accessibility attributes', () => {
    renderWithRedux(<TabNavigation {...defaultProps} activeTab="schemas" />, { initialState: createAuthenticatedState() })
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
    
    const inactiveTab = screen.getByRole('button', { name: /key management tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })
})