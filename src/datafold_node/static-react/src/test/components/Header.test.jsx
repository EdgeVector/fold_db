import { render, screen, fireEvent } from '@testing-library/react'
import { describe, it, expect, vi } from 'vitest'
import Header from '../../components/Header'

describe('Header Component', () => {
  it('renders header with correct title', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    expect(screen.getByText('DataFold Node')).toBeInTheDocument()
  })

  it('has correct styling classes', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const header = screen.getByRole('banner')
    expect(header).toHaveClass('bg-white', 'border-b', 'border-gray-200', 'shadow-sm', 'flex-shrink-0')
  })

  it('displays database SVG icon', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const svg = document.querySelector('svg')
    expect(svg).toBeInTheDocument()
    expect(svg).toHaveClass('w-8', 'h-8', 'flex-shrink-0')
  })

  it('has proper semantic structure', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const header = screen.getByRole('banner')
    expect(header).toBeInTheDocument()
    
    const link = screen.getByRole('link')
    expect(link).toBeInTheDocument()
    expect(link).toHaveAttribute('href', '/')
  })

  it('displays node status indicator', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const statusBadge = screen.getByText('Active').closest('.inline-flex')
    expect(statusBadge).toBeInTheDocument()
    expect(statusBadge).toHaveClass('inline-flex', 'items-center', 'gap-2', 'px-3', 'py-2', 'rounded-md', 'text-sm', 'font-medium', 'bg-green-100', 'text-green-800')
  })

  it('has proper layout classes', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const container = screen.getByRole('banner').firstChild
    expect(container).toHaveClass('flex', 'items-center', 'justify-between', 'px-6', 'py-3')
  })

  it('title link has hover effects', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const link = screen.getByRole('link')
    expect(link).toHaveClass('flex', 'items-center', 'gap-3', 'text-blue-600', 'hover:text-blue-700', 'transition-colors')
  })

  it('status indicator has green dot', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const statusContainer = screen.getByText('Active').parentElement
    const greenDot = statusContainer.querySelector('.bg-green-500')
    expect(greenDot).toBeInTheDocument()
    expect(greenDot).toHaveClass('w-2', 'h-2', 'rounded-full', 'bg-green-500')
  })

  it('title has correct typography classes', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const title = screen.getByText('DataFold Node')
    expect(title).toHaveClass('text-xl', 'font-semibold', 'text-gray-900')
  })

  it('renders settings button', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toBeInTheDocument()
  })

  it('calls onSettingsClick when settings button is clicked', () => {
    const mockSettingsClick = vi.fn()
    render(<Header onSettingsClick={mockSettingsClick} />)
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    fireEvent.click(settingsButton)
    
    expect(mockSettingsClick).toHaveBeenCalledTimes(1)
  })

  it('settings button has correct styling', () => {
    render(<Header onSettingsClick={vi.fn()} />)
    
    const settingsButton = screen.getByRole('button', { name: /settings/i })
    expect(settingsButton).toHaveClass('inline-flex', 'items-center', 'gap-2', 'px-3', 'py-2', 'text-sm', 'text-gray-700', 'hover:bg-gray-100', 'rounded-md', 'border', 'border-gray-300', 'transition-colors')
  })
})