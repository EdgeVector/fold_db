import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, it, expect, beforeEach, vi } from 'vitest'
import React from 'react'
import { useKeyGeneration } from '../../hooks/useKeyGeneration'
import KeyManagementTab from '../../components/tabs/KeyManagementTab'

// Mock deterministic Ed25519 functions
vi.mock('@noble/ed25519', () => ({
  utils: { randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)) },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3)))
}))

// Mock clipboard API - will be set up after userEvent.setup()
const mockWriteText = vi.fn(() => Promise.resolve())

function Wrapper() {
  const keyGen = useKeyGeneration()
  return <KeyManagementTab onResult={() => {}} keyGenerationResult={keyGen} />
}

describe('Key lifecycle workflow', () => {
  let user
  beforeEach(() => {
    user = userEvent.setup()
    
    // Set up clipboard mock after userEvent setup to avoid conflicts
    Object.defineProperty(navigator, 'clipboard', {
      value: {
        writeText: mockWriteText
      },
      writable: true,
      configurable: true
    })
    global.fetch = vi.fn((url, options) => {
      if (url === '/api/security/system-key') {
        if (options?.method === 'POST') {
          // Registration endpoint
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({
              success: true,
              public_key_id: 'test-key-id'
            })
          })
        } else {
          // GET endpoint - return existing system key
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({
              success: true,
              key: {
                public_key: 'AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=', // base64 of 32 bytes filled with 2
                id: 'SYSTEM_WIDE_PUBLIC_KEY'
              }
            })
          })
        }
      }
      if (url === '/api/mutation') {
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ ok: true }) })
      }
      return Promise.resolve({ ok: true, json: () => Promise.resolve({}) })
    })
    // Clear clipboard mock calls
    mockWriteText.mockClear()
  })

  it('generates, registers, signs and clears keys', async () => {
    render(<Wrapper />)
    await user.click(screen.getByText('Generate New Keypair'))

    await waitFor(() => {
      expect(screen.getByText('Register Public Key')).toBeInTheDocument()
    })

    await user.click(screen.getByText('Register Public Key'))
    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/security/system-key', expect.any(Object))
      expect(screen.getByText(/registered successfully/i)).toBeInTheDocument()
    })

    await user.type(screen.getByLabelText('Post ID'), 'post-abc')
    await user.type(screen.getByLabelText('Your User ID (Liker)'), 'user-xyz')
    await user.click(screen.getByRole('button', { name: /Sign and Send Like/i }))

    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith('/api/mutation', expect.any(Object))
    })

    await user.click(screen.getByText('Clear Keys'))
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /Sign and Send Like/i })).not.toBeInTheDocument()
    })
  })

  it('displays both public and private keys when generating a keypair', async () => {
    render(<Wrapper />)
    
    // Generate a new keypair
    await user.click(screen.getByText('Generate New Keypair'))

    // Wait for the key generation to complete and both keys to be displayed
    await waitFor(() => {
      expect(screen.getByText('Public Key (Base64) - Safe to share')).toBeInTheDocument()
    }, { timeout: 10000 })
    
    await waitFor(() => {
      expect(screen.getByText('Private Key (Base64) - Keep secret!')).toBeInTheDocument()
    }, { timeout: 10000 })
    
    await waitFor(() => {
      // Check that the keys themselves are displayed with expected values
      // Get all elements with the public key value (there will be 2: system key input and generated key textarea)
      const publicKeyElements = screen.getAllByDisplayValue('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
      expect(publicKeyElements).toHaveLength(2) // system key + generated key
      
      // Check for the generated public key specifically (should be a textarea)
      const publicKeyTextarea = publicKeyElements.find(el => el.tagName === 'TEXTAREA')
      expect(publicKeyTextarea).toBeInTheDocument()
      
      // Check for private key (should be unique)
      expect(screen.getByDisplayValue('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')).toBeInTheDocument()
      
      // Check that security warning is displayed
      expect(screen.getByText(/Never share your private key/i)).toBeInTheDocument()
    }, { timeout: 10000 })
  })

  it('allows copying both public and private keys with visual feedback', async () => {
    render(<Wrapper />)
    
    // Generate a new keypair
    await user.click(screen.getByText('Generate New Keypair'))

    await waitFor(() => {
      expect(screen.getByText('Public Key (Base64) - Safe to share')).toBeInTheDocument()
    })

    // Clear mock calls to start fresh
    mockWriteText.mockClear()
    
    // Get copy buttons by their titles
    const publicKeyCopyButton = screen.getByTitle('Copy public key')
    const privateKeyCopyButton = screen.getByTitle('Copy private key')

    // Test copying public key
    await user.click(publicKeyCopyButton)
    
    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledTimes(1)
      expect(mockWriteText).toHaveBeenCalledWith('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    })

    // Clear and test copying private key
    mockWriteText.mockClear()
    await user.click(privateKeyCopyButton)
    
    await waitFor(() => {
      expect(mockWriteText).toHaveBeenCalledTimes(1)
      expect(mockWriteText).toHaveBeenCalledWith('AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=')
    })
  })

  it('shows import private key section when system public key exists but no local keypair', async () => {
    render(<Wrapper />)
    
    await waitFor(() => {
      // Should show current system public key
      expect(screen.getByText('Current System Public Key:')).toBeInTheDocument()
      expect(screen.getByDisplayValue('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')).toBeInTheDocument()
      
      // Should show import private key section
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
      expect(screen.getByText(/You have a registered public key but no local private key/i)).toBeInTheDocument()
    })
  })

  it('validates private key import correctly', async () => {
    render(<Wrapper />)
    
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })

    // Click the import button to show the form
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter your private key here...')).toBeInTheDocument()
    })

    // Test with invalid private key
    await user.type(screen.getByPlaceholderText('Enter your private key here...'), 'invalid-key')
    await user.click(screen.getByText('Validate & Import'))
    
    await waitFor(() => {
      expect(screen.getByText(/Invalid private key format/i)).toBeInTheDocument()
    })

    // Clear the invalid input
    await user.clear(screen.getByPlaceholderText('Enter your private key here...'))
    
    // Test with valid private key that matches the system public key
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

    // Click the import button to show the form
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    
    await waitFor(() => {
      expect(screen.getByPlaceholderText('Enter your private key here...')).toBeInTheDocument()
    })

    // Type some text
    await user.type(screen.getByPlaceholderText('Enter your private key here...'), 'some-key-text')
    
    // Click cancel
    await user.click(screen.getByText('Cancel'))
    
    await waitFor(() => {
      // Form should be hidden and input cleared
      expect(screen.queryByPlaceholderText('Enter your private key here...')).not.toBeInTheDocument()
      // Import button should be visible again
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })
  })

  it('shows security warnings for private key handling', async () => {
    render(<Wrapper />)
    
    // Generate a new keypair to see private key security warning
    await user.click(screen.getByText('Generate New Keypair'))

    await waitFor(() => {
      expect(screen.getByText(/Never share your private key/i)).toBeInTheDocument()
      expect(screen.getByText(/Store it securely and only on trusted devices/i)).toBeInTheDocument()
    })

    // Clear keys to show import section
    await user.click(screen.getByText('Clear Keys'))
    
    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Import Private Key/i })).toBeInTheDocument()
    })

    // Click import to show security warning in import form
    await user.click(screen.getByRole('button', { name: /Import Private Key/i }))
    
    await waitFor(() => {
      expect(screen.getByText(/Only enter your private key on trusted devices/i)).toBeInTheDocument()
      expect(screen.getByText(/Never share or store private keys in plain text/i)).toBeInTheDocument()
    })
  })
})
