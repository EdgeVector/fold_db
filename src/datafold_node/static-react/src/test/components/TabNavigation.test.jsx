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

  it('renders all default tabs', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    DEFAULT_TABS.forEach(tab => {
      expect(screen.getByText(tab.label)).toBeInTheDocument()
    })
  })

  it('highlights active tab correctly', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} activeTab="schemas" />, { initialState: createAuthenticatedState() })
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
  })

  it('renders tabs without authentication labels when not authenticated', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`^${tab.label} tab$`, 'i') })
      expect(tabButton).toBeInTheDocument()
    })
  })

  it('shows check mark for Key Management tab when authenticated', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    const keysTab = screen.getByRole('button', { name: /key management tab/i })
    expect(keysTab).toBeInTheDocument()
  })

  it('keeps all tabs enabled regardless of authentication', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`^${tab.label} tab$`, 'i') })
      expect(tabButton).toBeEnabled()
    })
  })

  it('enables all tabs when authenticated', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`^${tab.label} tab$`, 'i') })
      expect(tabButton).toBeEnabled()
    })
  })

  it('calls onTabChange when clicking enabled tab', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    const schemasTab = screen.getByRole('button', { name: /schemas tab/i })
    fireEvent.click(schemasTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('schemas')
  })

  it('calls onTabChange when clicking any tab', async () => {
    const authRequiredTabs = [
      { id: 'admin', label: 'Admin', requiresAuth: true, icon: '👑' }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={authRequiredTabs} />, { initialState: createUnauthenticatedState() })
    
    const adminTab = screen.getByRole('button', { name: /admin tab/i })
    fireEvent.click(adminTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('admin')
  })

  it('allows clicking Key Management tab when not authenticated', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    
    const keysTab = screen.getByRole('button', { name: /key management tab/i })
    
    // Key Management tab should be enabled when not authenticated (doesn't require auth)
    expect(keysTab).not.toBeDisabled()
    fireEvent.click(keysTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('keys')
  })

  it('renders custom tabs when provided', async () => {
    const customTabs = [
      { id: 'custom1', label: 'Custom Tab 1', requiresAuth: false },
      { id: 'custom2', label: 'Custom Tab 2', requiresAuth: true }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={customTabs} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Custom Tab 1')).toBeInTheDocument()
    expect(screen.getByText('Custom Tab 2')).toBeInTheDocument()
    
    // Should not render default tabs
    expect(screen.queryByText('Schemas')).not.toBeInTheDocument()
  })

  it('displays tab icons when provided', async () => {
    const tabsWithIcons = [
      { id: 'test', label: 'Test Tab', requiresAuth: false, icon: '🧪' }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={tabsWithIcons} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('🧪')).toBeInTheDocument()
  })

  it('handles disabled tabs correctly', async () => {
    const tabsWithDisabled = [
      { id: 'enabled', label: 'Enabled Tab', requiresAuth: false, disabled: false },
      { id: 'disabled', label: 'Disabled Tab', requiresAuth: false, disabled: true }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={tabsWithDisabled} />, { initialState: createAuthenticatedState() })
    
    const enabledTab = screen.getByRole('button', { name: /enabled tab/i })
    const disabledTab = screen.getByRole('button', { name: /disabled tab/i })
    
    expect(enabledTab).toBeEnabled()
    expect(disabledTab).toBeDisabled()
  })

  it('applies custom className', async () => {
    const { container } = await renderWithRedux(
      <TabNavigation {...defaultProps} className="custom-nav" />,
      { initialState: createAuthenticatedState() }
    )
    
    expect(container.firstChild).toHaveClass('custom-nav')
  })

  it('has proper accessibility attributes', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} activeTab="schemas" />, { initialState: createAuthenticatedState() })
    
    const activeTab = screen.getByRole('button', { name: /schemas tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
    
    const inactiveTab = screen.getByRole('button', { name: /key management tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })
})