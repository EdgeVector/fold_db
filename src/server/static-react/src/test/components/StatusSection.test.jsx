import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import StatusSection from '../../components/StatusSection'

// Mock the systemClient
vi.mock('../../api/clients/systemClient', () => ({
  systemClient: {
    resetDatabase: vi.fn()
  }
}))

// Mock the ingestionClient 
vi.mock('../../api/clients', () => ({
  ingestionClient: {
    getAllProgress: vi.fn()
  }
}))

import { systemClient } from '../../api/clients/systemClient'
import { ingestionClient } from '../../api/clients'

describe('StatusSection Component', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    // Default mock implementation
    ingestionClient.getAllProgress.mockResolvedValue({
      success: true,
      data: [{
        id: '123',
        started_at: new Date().toISOString(),
        is_complete: false,
        status_message: 'Processing...',
        progress_percentage: 50
      }]
    })
  })

  it('renders system status heading', () => {
    render(<StatusSection />)
    
    expect(screen.getByText('System Status')).toBeInTheDocument()
  })

  it('has correct container styling', () => {
    render(<StatusSection />)
    
    const heading = screen.getByText('System Status')
    const container = heading.closest('.bg-white')
    expect(container).toHaveClass('bg-white', 'rounded-lg', 'shadow-sm', 'p-4', 'mb-6')
  })

  it('displays check circle icon', () => {
    render(<StatusSection />)
    
    // The CheckCircleIcon should be rendered as an SVG next to System Status
    const heading = screen.getByText('System Status')
    const icon = heading.parentElement.querySelector('svg')
    expect(icon).toBeInTheDocument()
    expect(icon).toHaveClass('w-5', 'h-5', 'text-green-500')
  })

  it('has proper layout structure', () => {
    render(<StatusSection />)
    
    const heading = screen.getByText('System Status')
    const headerContainer = heading.parentElement
    expect(headerContainer).toHaveClass('flex', 'items-center', 'gap-2')
  })

  it('renders ingestion status card', async () => {
    render(<StatusSection />)
    
    // Check for Ingestion text in the status cards
    await waitFor(() => {
      expect(screen.getByText(/Ingestion/i)).toBeInTheDocument()
    })
  })

  it('renders indexing status card', () => {
    render(<StatusSection />)
    
    // Check for Indexing text in the status cards
    const indexingElements = screen.getAllByText(/Indexing/i)
    expect(indexingElements.length).toBeGreaterThan(0)
  })

  it('renders all visual elements', async () => {
    render(<StatusSection />)
    
    // Check that all key elements are present
    expect(screen.getByText('System Status')).toBeInTheDocument()
    
    await waitFor(() => {
        expect(screen.getByText(/Ingestion/i)).toBeInTheDocument()
    })
    
    const indexingElements = screen.getAllByText(/Indexing/i)
    expect(indexingElements.length).toBeGreaterThan(0)
    
    // Check for icon
    const heading = screen.getByText('System Status')
    const icon = heading.parentElement.querySelector('svg')
    expect(icon).toBeInTheDocument()
  })

  describe('Database Reset Functionality', () => {
    beforeEach(() => {
      // Reset all mocks before each test
      vi.clearAllMocks()
    })

    it('renders reset database button', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      expect(resetButton).toBeInTheDocument()
      expect(resetButton).toHaveClass('text-red-600', 'border-red-200')
    })

    it('shows confirmation dialog when reset button is clicked', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      expect(screen.getByRole('heading', { name: /reset database/i })).toBeInTheDocument()
      expect(screen.getByText(/This will permanently delete all data/)).toBeInTheDocument()
      expect(screen.getByText(/All schemas will be removed/)).toBeInTheDocument()
      expect(screen.getByText(/This action cannot be undone/)).toBeInTheDocument()
    })

    it('closes confirmation dialog when cancel is clicked', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const cancelButton = screen.getByRole('button', { name: /cancel/i })
      fireEvent.click(cancelButton)
      
      expect(screen.queryByRole('heading', { name: /reset database/i })).not.toBeInTheDocument()
    })

    it('calls systemClient when reset is confirmed', async () => {
      systemClient.resetDatabase.mockResolvedValueOnce({
        success: true,
        data: { success: true, message: 'Database reset successfully' }
      })

      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1]
      fireEvent.click(confirmButton)
      
      await waitFor(() => {
        expect(systemClient.resetDatabase).toHaveBeenCalledWith(true)
      })
    })

    it('shows success message when reset succeeds', async () => {
      systemClient.resetDatabase.mockResolvedValueOnce({
        success: true,
        data: { success: true, message: 'Database reset successfully' }
      })

      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1] // Get the modal button
      fireEvent.click(confirmButton)
      
      await waitFor(() => {
        expect(screen.getByText('Database reset successfully')).toBeInTheDocument()
      })
    })

    it('shows error message when reset fails', async () => {
      systemClient.resetDatabase.mockResolvedValueOnce({
        success: false,
        error: 'Reset failed'
      })

      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1] // Get the modal button
      fireEvent.click(confirmButton)
      
      await waitFor(() => {
        expect(screen.getByText('Reset failed')).toBeInTheDocument()
      })
    })

    it('handles network errors gracefully', async () => {
      systemClient.resetDatabase.mockRejectedValueOnce(new Error('Network error'))

      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1] // Get the modal button
      fireEvent.click(confirmButton)
      
      await waitFor(() => {
        expect(screen.getByText(/Network error/)).toBeInTheDocument()
      })
    })

    it('disables reset button while resetting', async () => {
      systemClient.resetDatabase.mockImplementationOnce(() => new Promise(resolve => setTimeout(resolve, 1000)))

      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1] // Get the modal button
      fireEvent.click(confirmButton)
      
      // Button should show "Resetting..." and be disabled
      await waitFor(() => {
        expect(screen.getByText('Resetting...')).toBeInTheDocument()
      })
      
      const disabledButton = screen.getByRole('button', { name: /resetting/i })
      expect(disabledButton).toBeDisabled()
    })

    it('shows proper button styling for destructive action', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      const confirmButton = screen.getAllByRole('button', { name: /reset database/i })[1] // Get the modal button
      expect(confirmButton).toHaveClass('bg-red-600', 'text-white', 'hover:bg-red-700')
    })

    it('includes trash icon in reset button', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      const icon = resetButton.querySelector('svg')
      expect(icon).toBeInTheDocument()
      expect(icon).toHaveClass('w-4', 'h-4')
    })

    it('confirms dialog accessibility features', () => {
      render(<StatusSection />)
      
      const resetButton = screen.getByRole('button', { name: /reset database/i })
      fireEvent.click(resetButton)
      
      // Check for proper heading in the dialog
      const dialogHeadings = screen.getAllByRole('heading', { level: 3 })
      const resetDialogHeading = dialogHeadings.find(h => h.textContent === 'Reset Database')
      expect(resetDialogHeading).toBeDefined()
      expect(resetDialogHeading).toHaveTextContent('Reset Database')
      
      // Check for proper button roles
      expect(screen.getByRole('button', { name: /cancel/i })).toBeInTheDocument()
      expect(screen.getAllByRole('button', { name: /reset database/i })[1]).toBeInTheDocument() // Get the modal button
    })
  })
})