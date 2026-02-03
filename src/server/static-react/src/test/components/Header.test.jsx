import { screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import Header from '../../components/Header'
import { renderWithRedux, createMockAuthState } from '../utils/testStore.jsx'

describe('Header Component', () => {
  const defaultPreloadedState = {
    auth: createMockAuthState()
  }

  it('renders header with correct title', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    // Header shows "datafold"
    expect(screen.getByText(/datafold/i)).toBeInTheDocument()
  })

  it('has minimal header styling', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    const header = screen.getByRole('banner')
    expect(header).toHaveClass('minimal-header', 'flex-shrink-0')
  })

  it('has proper semantic structure', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    const header = screen.getByRole('banner')
    expect(header).toBeInTheDocument()

    const link = screen.getByRole('link')
    expect(link).toBeInTheDocument()
    expect(link).toHaveAttribute('href', '/')
  })

  it('has proper layout classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    const container = screen.getByRole('banner').firstChild
    expect(container).toHaveClass('flex', 'items-center', 'justify-between')
  })

  it('title link has minimal logo class', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    const link = screen.getByRole('link')
    expect(link).toHaveClass('minimal-logo')
  })

  it('displays settings button', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toBeInTheDocument()
  })

  it('displays connected status', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })

    expect(screen.getByText('Connected')).toBeInTheDocument()
  })

  it('calls onSettingsClick when settings button is clicked', () => {
    const mockSettingsClick = vi.fn()
    renderWithRedux(<Header onSettingsClick={mockSettingsClick} />, {
      preloadedState: defaultPreloadedState
    })

    const settingsButton = screen.getByRole('button', { name: /settings/i })
    fireEvent.click(settingsButton)

    expect(mockSettingsClick).toHaveBeenCalledTimes(1)
  })

  it('shows user info when authenticated', () => {
    const authenticatedState = {
      auth: createMockAuthState({ isAuthenticated: true, user: { id: 'testuser' } })
    }
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: authenticatedState
    })

    expect(screen.getByText('testuser')).toBeInTheDocument()
    expect(screen.getByText('logout')).toBeInTheDocument()
  })
})
