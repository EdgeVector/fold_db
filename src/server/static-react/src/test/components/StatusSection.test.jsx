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
    const container = heading.closest('.card')
    expect(container).toHaveClass('card', 'p-4', 'mb-6')
  })

  it('displays check circle icon', () => {
    render(<StatusSection />)
    
    // The CheckCircleIcon should be rendered as an SVG next to System Status
    const heading = screen.getByText('System Status')
    const icon = heading.parentElement.querySelector('svg')
    expect(icon).toBeInTheDocument()
    expect(icon).toHaveClass('w-5', 'h-5', 'text-gruvbox-green')
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

  it('renders indexing job card when indexing job is returned', async () => {
    // Mock an indexing job response
    ingestionClient.getAllProgress.mockResolvedValue({
      success: true,
      data: [{
        id: 'idx-123',
        started_at: new Date().toISOString(),
        is_complete: false,
        status_message: 'Indexing documents...',
        progress_percentage: 75,
        job_type: 'indexing'
      }]
    })
    
    render(<StatusSection />)
    
    // Check for Indexing text in the status cards
    await waitFor(() => {
      expect(screen.getByText(/Indexing Job/i)).toBeInTheDocument()
    })
  })

  it('renders all visual elements', async () => {
    render(<StatusSection />)
    
    // Check that all key elements are present
    expect(screen.getByText('System Status')).toBeInTheDocument()
    
    // Wait for the ingestion job from the mock to render
    await waitFor(() => {
        expect(screen.getByText(/Ingestion Job/i)).toBeInTheDocument()
    })
    
    // Check for icon
    const heading = screen.getByText('System Status')
    const icon = heading.parentElement.querySelector('svg')
    expect(icon).toBeInTheDocument()
  })

  it('shows "No active jobs" placeholder when no jobs are returned', async () => {
    ingestionClient.getAllProgress.mockResolvedValue({
      success: true,
      data: []
    })
    
    render(<StatusSection />)
    
    await waitFor(() => {
      expect(screen.getByText('No active jobs')).toBeInTheDocument()
    })
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
      expect(resetButton).toHaveClass('btn-danger', 'btn-sm')
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
      expect(confirmButton).toHaveClass('btn-danger')
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