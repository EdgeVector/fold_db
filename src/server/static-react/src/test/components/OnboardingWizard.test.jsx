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
        openrouter: { api_key: '', model: 'google/gemini-2.0-flash-001', base_url: '' },
        ollama: { model: 'llama3.1:8b', base_url: '' },
      },
    }),
    saveConfig: vi.fn().mockResolvedValue({
      success: true,
      data: { success: true, message: 'Saved' },
    }),
  },
  systemClient: {
    getDatabaseConfig: vi.fn().mockResolvedValue({
      success: true,
      data: { type: 'local', path: '/tmp/fold_db' },
    }),
  },
}))

vi.mock('../../components/form/SelectField', () => ({
  default: ({ name, label, value, options, onChange }) => (
    <div data-testid={`select-${name}`}>
      <label>{label}</label>
      <select
        data-testid={`select-input-${name}`}
        value={value}
        onChange={(e) => onChange(e.target.value)}
      >
        {options.map((o) => (
          <option key={o.value} value={o.value}>{o.label}</option>
        ))}
      </select>
    </div>
  ),
}))

const { ingestionClient } = await import('../../api/clients')

describe('OnboardingWizard', () => {
  const mockOnClose = vi.fn()

  beforeEach(() => {
    vi.clearAllMocks()
    localStorage.clear()
  })

  it('renders welcome step when open', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByText('Welcome to FoldDB')).toBeInTheDocument()
    })
    expect(screen.getByText('Get Started')).toBeInTheDocument()
    expect(screen.getByText('Step 1 of 4')).toBeInTheDocument()
  })

  it('does not render when closed', () => {
    renderWithRedux(<OnboardingWizard isOpen={false} onClose={mockOnClose} />)

    expect(screen.queryByText('Welcome to FoldDB')).not.toBeInTheDocument()
  })

  it('advances from welcome to configure AI', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByText('Welcome to FoldDB')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('Get Started'))

    await waitFor(() => {
      expect(screen.getByText('Configure AI Provider')).toBeInTheDocument()
    })
    expect(screen.getByText('Step 2 of 4')).toBeInTheDocument()
  })

  it('marks completed when skipping tutorial', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    await waitFor(() => {
      expect(screen.getByText('Skip Tutorial')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('Skip Tutorial'))

    expect(localStorage.getItem(BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED)).toBe('1')
    expect(mockOnClose).toHaveBeenCalled()
  })

  it('saves AI config and advances', async () => {
    vi.useFakeTimers()

    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    // Go to step 2
    await waitFor(() => {
      expect(screen.getByText('Get Started')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Get Started'))

    await waitFor(() => {
      expect(screen.getByText('Configure AI Provider')).toBeInTheDocument()
    })

    // Enter API key
    const apiKeyInput = screen.getByTestId('api-key-input')
    fireEvent.change(apiKeyInput, { target: { value: 'sk-or-test-key' } })

    // Click save
    fireEvent.click(screen.getByText('Save & Continue'))

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
      expect(screen.getByText('Storage Configuration')).toBeInTheDocument()
    })

    vi.useRealTimers()
  })

  it('displays storage mode', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    // Navigate to step 3
    await waitFor(() => {
      expect(screen.getByText('Get Started')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Get Started'))

    await waitFor(() => {
      expect(screen.getByText('Configure AI Provider')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Skip for Now'))

    await waitFor(() => {
      expect(screen.getByText('Storage Configuration')).toBeInTheDocument()
    })
    expect(screen.getByText('Local Storage')).toBeInTheDocument()
  })

  it('completes wizard on final step', async () => {
    renderWithRedux(<OnboardingWizard isOpen={true} onClose={mockOnClose} />)

    // Step 1 -> 2
    await waitFor(() => {
      expect(screen.getByText('Get Started')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Get Started'))

    // Step 2 -> 3
    await waitFor(() => {
      expect(screen.getByText('Configure AI Provider')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Skip for Now'))

    // Step 3 -> 4
    await waitFor(() => {
      expect(screen.getByText('Storage Configuration')).toBeInTheDocument()
    })
    fireEvent.click(screen.getByText('Continue'))

    // Step 4 (Done)
    await waitFor(() => {
      expect(screen.getByText("You're All Set!")).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('Start Using FoldDB'))

    expect(localStorage.getItem(BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED)).toBe('1')
    expect(mockOnClose).toHaveBeenCalled()
  })
})
