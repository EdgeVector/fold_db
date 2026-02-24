import React from 'react'
import { screen, fireEvent, waitFor, act } from '@testing-library/react'
import { vi, describe, it, expect, beforeEach } from 'vitest'
import OnboardingWizard from '../../components/OnboardingWizard'
import { renderWithRedux } from '../utils/testHelpers.jsx'
import { BROWSER_CONFIG } from '../../constants/config'

vi.mock('../../api/clients', () => ({
  ingestionClient: {
    getConfig: vi.fn().mockResolvedValue({
      success: true,
      data: {
        provider: 'OpenRouter',
        openrouter: { api_key: '', model: 'google/gemini-2.5-flash', base_url: '' },
        ollama: { model: 'llama3.1:8b', base_url: '' },
      },
    }),
    saveConfig: vi.fn().mockResolvedValue({
      success: true,
      data: { success: true, message: 'Saved' },
    }),
    uploadFile: vi.fn().mockResolvedValue({
      success: true,
      data: { schema_name: 'TestSchema', new_schema_created: true, mutations_executed: 1 },
    }),
    smartFolderScan: vi.fn().mockResolvedValue({
      success: true,
      data: { total_files: 10, recommendations: [] },
    }),
  },
  llmQueryClient: {
    agentQuery: vi.fn().mockResolvedValue({
      data: { answer: 'Test answer' },
    }),
  },
}))

const { ingestionClient } = await import('../../api/clients')

describe('OnboardingWizard', () => {
  const mockOnClose = vi.fn()
  const testUserHash = 'abc123testhash'
  const onboardingKey = `${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${testUserHash}`

  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('renders welcome step when open', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    await waitFor(() => {
      expect(screen.getByText('Welcome to FoldDB')).toBeInTheDocument()
    })
    expect(screen.getByText('[Get Started]')).toBeInTheDocument()
    expect(screen.getByText('Step 1 of 6')).toBeInTheDocument()
  })

  it('does not render when closed', () => {
    renderWithRedux(<OnboardingWizard isOpen={false} onClose={mockOnClose} userHash={testUserHash} />)

    expect(screen.queryByText('Welcome to FoldDB')).not.toBeInTheDocument()
  })

  it('advances from welcome to configure AI', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    await waitFor(() => {
      expect(screen.getByText('Welcome to FoldDB')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('[Get Started]'))

    await waitFor(() => {
      expect(screen.getByText('CONFIGURE AI')).toBeInTheDocument()
    })
    expect(screen.getByText('Step 2 of 6')).toBeInTheDocument()
  })

  it('marks completed when skipping tutorial', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    await waitFor(() => {
      expect(screen.getByText('Skip Tutorial')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('Skip Tutorial'))

    expect(localStorage.getItem(onboardingKey)).toBe('1')
    expect(mockOnClose).toHaveBeenCalled()
  })

  it('saves AI config and advances to first file step', async () => {
    vi.useFakeTimers()

    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    // Go to step 2
    await waitFor(() => {
      expect(screen.getByText('[Get Started]')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Get Started]'))

    await waitFor(() => {
      expect(screen.getByText('CONFIGURE AI')).toBeInTheDocument()
    })

    // Enter API key
    const apiKeyInput = screen.getByTestId('api-key-input')
    fireEvent.change(apiKeyInput, { target: { value: 'sk-or-test-key' } })

    // Click save
    fireEvent.click(screen.getByText('[Save & Continue]'))

    await waitFor(() => {
      expect(ingestionClient.saveConfig).toHaveBeenCalled()
    })

    // Should show success message
    await waitFor(() => {
      expect(screen.getByText('Configuration saved successfully!')).toBeInTheDocument()
    })

    // Fast-forward the 1s auto-advance timer
    act(() => {
      vi.advanceTimersByTime(1100)
    })

    await waitFor(() => {
      expect(screen.getByText('FIRST FILE')).toBeInTheDocument()
    })

    vi.useRealTimers()
  })

  it('skips through all steps to done', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    // Step 1 -> 2
    await waitFor(() => {
      expect(screen.getByText('[Get Started]')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Get Started]'))

    // Step 2 -> 3
    await waitFor(() => {
      expect(screen.getByText('CONFIGURE AI')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Skip]'))

    // Step 3 -> 4
    await waitFor(() => {
      expect(screen.getByText('FIRST FILE')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Skip for now]'))

    // Step 4 -> 5
    await waitFor(() => {
      expect(screen.getByText('AI QUERY')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Skip]'))

    // Step 5 -> 6
    await waitFor(() => {
      expect(screen.getByText('SMART FOLDER')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('[Skip]'))

    // Step 6 (Done)
    await waitFor(() => {
      expect(screen.getByText("You're all set.")).toBeInTheDocument()
    })
  })

  it('completes wizard on final step', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} userHash={testUserHash} />)

    // Navigate through all steps to Done
    await waitFor(() => { expect(screen.getByText('[Get Started]')).toBeInTheDocument() })
    fireEvent.click(screen.getByText('[Get Started]'))

    await waitFor(() => { expect(screen.getByText('CONFIGURE AI')).toBeInTheDocument() })
    fireEvent.click(screen.getByText('[Skip]'))

    await waitFor(() => { expect(screen.getByText('FIRST FILE')).toBeInTheDocument() })
    fireEvent.click(screen.getByText('[Skip for now]'))

    await waitFor(() => { expect(screen.getByText('AI QUERY')).toBeInTheDocument() })
    fireEvent.click(screen.getByText('[Skip]'))

    await waitFor(() => { expect(screen.getByText('SMART FOLDER')).toBeInTheDocument() })
    fireEvent.click(screen.getByText('[Skip]'))

    await waitFor(() => {
      expect(screen.getByText("You're all set.")).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('[Start Using FoldDB]'))

    expect(localStorage.getItem(onboardingKey)).toBe('1')
    expect(mockOnClose).toHaveBeenCalled()
  })
})
