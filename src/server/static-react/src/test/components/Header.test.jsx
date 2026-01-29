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
    
    // Header now shows "$ fold_db" - look for fold_db text
    expect(screen.getByText(/fold_db/i)).toBeInTheDocument()
  })

  it('has correct terminal styling classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const header = screen.getByRole('banner')
    expect(header).toHaveClass('bg-terminal-lighter', 'border-b', 'border-terminal', 'flex-shrink-0')
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
    expect(container).toHaveClass('flex', 'items-center', 'justify-between', 'px-6', 'py-3')
  })

  it('title link has terminal hover effects', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const link = screen.getByRole('link')
    expect(link).toHaveClass('flex', 'items-center', 'gap-3', 'text-terminal-green')
  })

  it('displays config button', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const configButton = screen.getByRole('button', { name: /config/i })
    expect(configButton).toBeInTheDocument()
    expect(configButton).toHaveClass('btn-terminal')
  })

  it('config button contains settings icon', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const configButton = screen.getByRole('button', { name: /config/i })
    const icon = configButton.querySelector('svg')
    expect(icon).toBeInTheDocument()
    expect(icon).toHaveClass('w-4', 'h-4')
  })

  it('title has correct typography classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    // Find the span containing the title text
    const titleSpan = screen.getByText(/fold_db/i).closest('span')
    expect(titleSpan).toHaveClass('text-xl', 'font-bold', 'tracking-tight')
  })

  it('calls onSettingsClick when config button is clicked', () => {
    const mockSettingsClick = vi.fn()
    renderWithRedux(<Header onSettingsClick={mockSettingsClick} />, {
      preloadedState: defaultPreloadedState
    })
    
    const configButton = screen.getByRole('button', { name: /config/i })
    fireEvent.click(configButton)
    
    expect(mockSettingsClick).toHaveBeenCalledTimes(1)
  })

  it('displays version badge', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    expect(screen.getByText('v1.0')).toBeInTheDocument()
  })

  it('shows user info when authenticated', () => {
    const authenticatedState = {
      auth: createMockAuthState({ isAuthenticated: true, user: { id: 'testuser' } })
    }
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: authenticatedState
    })
    
    expect(screen.getByText('testuser')).toBeInTheDocument()
    expect(screen.getByText('exit')).toBeInTheDocument()
  })
})