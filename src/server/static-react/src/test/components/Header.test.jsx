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
    
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
  })

  it('has correct styling classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const header = screen.getByRole('banner')
    expect(header).toHaveClass('bg-white', 'border-b', 'border-gray-200', 'shadow-sm', 'flex-shrink-0')
  })

  it('displays database SVG icon', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const svg = document.querySelector('svg')
    expect(svg).toBeInTheDocument()
    expect(svg).toHaveClass('w-8', 'h-8', 'flex-shrink-0')
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

  it('title link has hover effects', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const link = screen.getByRole('link')
    expect(link).toHaveClass('flex', 'items-center', 'gap-3', 'text-blue-600', 'hover:text-blue-700', 'transition-colors')
  })

  it('displays settings button with correct classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toHaveClass('inline-flex', 'items-center', 'gap-2', 'px-3', 'py-2', 'text-sm')
  })

  it('settings button contains icon', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    const icon = settingsButton.querySelector('svg')
    expect(icon).toBeInTheDocument()
    expect(icon).toHaveClass('w-4', 'h-4')
  })

  it('title has correct typography classes', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const title = screen.getByText('DataFold Node')
    expect(title).toHaveClass('text-xl', 'font-semibold', 'text-gray-900')
  })

  it('renders settings button', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toBeInTheDocument()
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

  it('settings button has correct styling', () => {
    renderWithRedux(<Header onSettingsClick={vi.fn()} />, {
      preloadedState: defaultPreloadedState
    })
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toHaveClass('inline-flex', 'items-center', 'gap-2', 'px-3', 'py-2', 'text-sm', 'text-gray-700', 'hover:bg-gray-100', 'rounded-md', 'border', 'border-gray-300', 'transition-colors')
  })
})