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
    activeTab: 'ingestion',
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

  it('renders main group tabs', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Ingestion')).toBeInTheDocument()
    expect(screen.getByText('AI Query')).toBeInTheDocument()
  })

  it('renders advanced group tabs', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })

    // Current advanced tab in DEFAULT_TABS
    expect(screen.getByText('Native Index Query')).toBeInTheDocument()
  })

  it('displays Advanced label for advanced group', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Advanced')).toBeInTheDocument()
  })

  it('highlights active tab correctly', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} activeTab="ingestion" />, { initialState: createAuthenticatedState() })

    const activeTab = screen.getByRole('button', { name: /ingestion tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')
  })

  it('renders tabs without authentication labels when not authenticated', async () => {
    await renderWithRedux(<TabNavigation {...defaultProps} />, { initialState: createUnauthenticatedState() })
    DEFAULT_TABS.forEach(tab => {
      const tabButton = screen.getByRole('button', { name: new RegExp(`^${tab.label} tab$`, 'i') })
      expect(tabButton).toBeInTheDocument()
    })
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

    const aiQueryTab = screen.getByRole('button', { name: /ai query tab/i })
    fireEvent.click(aiQueryTab)

    expect(defaultProps.onTabChange).toHaveBeenCalledWith('llm-query')
  })

  it('calls onTabChange when clicking any tab', async () => {
    const customTabs = [
      { id: 'custom', label: 'Custom', requiresAuth: false, icon: '⚡', group: 'main' }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={customTabs} />, { initialState: createUnauthenticatedState() })
    
    const customTab = screen.getByRole('button', { name: /custom tab/i })
    fireEvent.click(customTab)
    
    expect(defaultProps.onTabChange).toHaveBeenCalledWith('custom')
  })

  it('renders custom tabs when provided', async () => {
    const customTabs = [
      { id: 'custom1', label: 'Custom Tab 1', requiresAuth: false, group: 'main' },
      { id: 'custom2', label: 'Custom Tab 2', requiresAuth: true, group: 'main' }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={customTabs} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('Custom Tab 1')).toBeInTheDocument()
    expect(screen.getByText('Custom Tab 2')).toBeInTheDocument()
    
    // Should not render default tabs
    expect(screen.queryByText('Schemas')).not.toBeInTheDocument()
  })

  it('displays tab icons when provided', async () => {
    const tabsWithIcons = [
      { id: 'test', label: 'Test Tab', requiresAuth: false, icon: '🧪', group: 'main' }
    ]
    
    await renderWithRedux(<TabNavigation {...defaultProps} tabs={tabsWithIcons} />, { initialState: createAuthenticatedState() })
    
    expect(screen.getByText('🧪')).toBeInTheDocument()
  })

  it('handles disabled tabs correctly', async () => {
    const tabsWithDisabled = [
      { id: 'enabled', label: 'Enabled Tab', requiresAuth: false, disabled: false, group: 'main' },
      { id: 'disabled', label: 'Disabled Tab', requiresAuth: false, disabled: true, group: 'main' }
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
    await renderWithRedux(<TabNavigation {...defaultProps} activeTab="ingestion" />, { initialState: createAuthenticatedState() })

    const activeTab = screen.getByRole('button', { name: /ingestion tab/i })
    expect(activeTab).toHaveAttribute('aria-current', 'page')

    const inactiveTab = screen.getByRole('button', { name: /ai query tab/i })
    expect(inactiveTab).not.toHaveAttribute('aria-current')
  })
})