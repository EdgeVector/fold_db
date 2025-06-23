import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import React from 'react'
import { useKeyGeneration } from '../../hooks/useKeyGeneration'
import KeyManagementTab from '../../components/tabs/KeyManagementTab'
import { AuthenticationProvider } from '../../auth/useAuth'
import { setupAuthTestEnvironment, createClipboardMock } from '../utils/authMocks'

// Mock Ed25519 functions
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

function Wrapper() {
  const keyGen = useKeyGeneration()
  return (
    <AuthenticationProvider>
      <KeyManagementTab onResult={() => {}} keyGenerationResult={keyGen} />
    </AuthenticationProvider>
  )
}

describe('Key lifecycle workflow', () => {
  let user
  let mockWriteText

  beforeEach(() => {
    user = userEvent.setup()
    setupAuthTestEnvironment()
    mockWriteText = createClipboardMock()

    // Extended fetch mock for full workflow testing
    global.fetch = vi.fn((url, options) => {
      if (url === '/api/security/system-key') {
        if (options?.method === 'POST') {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({
              success: true,
              public_key_id: 'test-key-id'
            })
          })
        } else {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({
              success: true,
              key: {
                public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=',
                id: 'SYSTEM_WIDE_PUBLIC_KEY'
              }
            })
          })
        }
      }
      if (url === '/api/schemas') {
        return Promise.resolve({
          ok: true,
          json: () => Promise.resolve({
            success: true,
            data: { 'TransformBase': 'approved' }
          })
        })
      }
      if (url === '/api/mutation') {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ ok: true }) })
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({}) })
    })

    mockWriteText.mockClear()
  })

  it('completes full key lifecycle: generate → register → sign → clear', async () => {
    render(<Wrapper />)
    
    // Generate new keypair
    await user.click(screen.getByText('Generate New Keypair'))
    await waitFor(() => {
      expect(screen.getByText('Register Public Key')).toBeInTheDocument()
    })

    // Register the public key
    await user.click(screen.getByText('Register Public Key'))
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/security/system-key', expect.any(Object))
      expect(screen.getByText(/registered successfully/i)).toBeInTheDocument()
    })

    // Wait for DataStorageForm to be ready
    await waitFor(() => {
      expect(screen.getByText('Selected Schema')).toBeInTheDocument()
    })
    
    await waitFor(() => {
      const submitButton = screen.getByRole('button', { name: /Sign and Submit Transform Data/i })
      expect(submitButton).not.toBeDisabled()
    })

    // Use the authenticated keys to sign data
    await user.type(screen.getByLabelText('Value 1'), 'post-abc')
    await user.type(screen.getByLabelText('Value 2'), 'user-xyz')
    await user.click(screen.getByRole('button', { name: /Sign and Submit Transform Data/i }))

    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/mutation', expect.any(Object))
    }, { timeout: 5000 })

    // Clear keys to complete lifecycle
    await user.click(screen.getByText('Clear Keys'))
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /Sign and Submit Transform Data/i })).not.toBeInTheDocument()
    })
  })

  it('displays both public and private keys when generating a keypair', async () => {
    render(<Wrapper />)
    
    await user.click(screen.getByText('Generate New Keypair'))

    await waitFor(() => {
      expect(screen.getByText('Public Key (Base64) - Safe to share')).toBeInTheDocument()
      expect(screen.getByText('Private Key (Base64) - Keep secret!')).toBeInTheDocument()
      
      // Check that keys are displayed with expected values
      const publicKeyElements = screen.getAllByDisplayValue('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
      expect(publicKeyElements).toHaveLength(2) // system key + generated key
      
      const publicKeyTextarea = publicKeyElements.find(el => el.tagName === 'TEXTAREA')
      expect(publicKeyTextarea).toBeInTheDocument()
      
      expect(screen.getByDisplayValue('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')).toBeInTheDocument()
      expect(screen.getByText(/Never share your private key/i)).toBeInTheDocument()
    }, { timeout: 10000 })
  })

  it('allows copying both public and private keys with visual feedback', async () => {
    render(<Wrapper />)
    
    await user.click(screen.getByText('Generate New Keypair'))
    await waitFor(() => {
      expect(screen.getByText('Public Key (Base64) - Safe to share')).toBeInTheDocument()
    })

    mockWriteText.mockClear()
    
    // Test copying keys
    await user.click(screen.getByTitle('Copy public key'))
    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledWith('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    })

    mockWriteText.mockClear()
    await user.click(screen.getByTitle('Copy private key'))
    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledWith('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')
    })
  })

  it('handles private key import workflow', async () => {
    render(<Wrapper />)
    
    await waitFor(() => {
      expect(screen.getByText('Current System Public Key:')).toBeInTheDocument()
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
      expect(screen.getByText(/You have a registered public key but no local private key/i)).toBeInTheDocument()
    })

    // Open import form
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter your private key here...')).toBeInTheDocument()
    })

    // Test with invalid private key
    await user.type(screen.getByPlaceholderText('Enter your private key here...'), 'invalid-key')
    await user.click(screen.getByText('Validate & Import'))
    await waitFor(() => {
      expect(screen.getByText(/Private key does not match the system public key/i)).toBeInTheDocument()
    })

    // Test with valid private key
    await user.clear(screen.getByPlaceholderText('Enter your private key here...'))
    await user.type(screen.getByPlaceholderText('Enter your private key here...'), 'AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')
    await user.click(screen.getByText('Validate & Import'))
    await waitFor(() => {
      expect(screen.getByText(/Private key matches system public key/i)).toBeInTheDocument()
    })
  })

  it('handles private key import cancellation', async () => {
    render(<Wrapper />)
    
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })

    // Open import form and type some text
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter your private key here...')).toBeInTheDocument()
    })
    await user.type(screen.getByPlaceholderText('Enter your private key here...'), 'some-key-text')
    
    // Cancel import
    await user.click(screen.getByText('Cancel'))
    await waitFor(() => {
      expect(screen.queryByPlaceholderText('Enter your private key here...')).not.toBeInTheDocument()
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })
  })

  it('shows security warnings for private key handling', async () => {
    render(<Wrapper />)
    
    // Security warnings in key generation
    await user.click(screen.getByText('Generate New Keypair'))
    await waitFor(() => {
      expect(screen.getByText(/Never share your private key/i)).toBeInTheDocument()
      expect(screen.getByText(/Store it securely and only on trusted devices/i)).toBeInTheDocument()
    })

    // Security warnings in import
    await user.click(screen.getByText('Clear Keys'))
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })
    
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    await waitFor(() => {
      expect(screen.getByText(/Only enter your private key on trusted devices/i)).toBeInTheDocument()
      expect(screen.getByText(/Never share or store private keys in plain text/i)).toBeInTheDocument()
    })
  })
})
