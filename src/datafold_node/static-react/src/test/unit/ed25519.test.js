import { describe, it, expect, vi } from 'vitest'
import {
  bytesToBase64,
  base64ToBytes,
  hexToBytes,
  bytesToHex,
  sign,
  generateEd25519KeyPair
} from '../../utils/ed25519'
import { createEd25519Mocks } from '../utils/authMocks'

// Mock @noble/ed25519
vi.mock('@noble/ed25519', () => createEd25519Mocks())

describe('Ed25519 Utility Functions', () => {
  describe('bytesToBase64', () => {
    it('converts Uint8Array to base64 string', () => {
      const bytes = new Uint8Array([72, 101, 108, 108, 111]) // "Hello"
      const result = bytesToBase64(bytes)
      expect(result).toBe('SGVsbG8=')
    })

    it('handles empty array', () => {
      const bytes = new Uint8Array([])
      const result = bytesToBase64(bytes)
      expect(result).toBe('')
    })
  })

  describe('base64ToBytes', () => {
    it('converts base64 string to Uint8Array', () => {
      const base64 = 'SGVsbG8='
      const result = base64ToBytes(base64)
      expect(result).toEqual(new Uint8Array([72, 101, 108, 108, 111]))
    })

    it('handles empty string', () => {
      const base64 = ''
      const result = base64ToBytes(base64)
      expect(result).toEqual(new Uint8Array([]))
    })

    it('throws error for invalid base64', () => {
      expect(() => {
        base64ToBytes('invalid-base64!')
      }).toThrow()
    })
  })

  describe('hexToBytes', () => {
    it('converts hex string to Uint8Array', () => {
      const hex = '48656c6c6f'
      const result = hexToBytes(hex)
      expect(result).toEqual(new Uint8Array([72, 101, 108, 108, 111]))
    })

    it('handles uppercase hex', () => {
      const hex = '48656C6C6F'
      const result = hexToBytes(hex)
      expect(result).toEqual(new Uint8Array([72, 101, 108, 108, 111]))
    })

    it('handles empty string', () => {
      const hex = ''
      const result = hexToBytes(hex)
      expect(result).toEqual(new Uint8Array([]))
    })
  })

  describe('bytesToHex', () => {
    it('converts Uint8Array to hex string', () => {
      const bytes = new Uint8Array([72, 101, 108, 108, 111])
      const result = bytesToHex(bytes)
      expect(result).toBe('48656c6c6f')
    })

    it('pads single digit hex values', () => {
      const bytes = new Uint8Array([1, 15, 255])
      const result = bytesToHex(bytes)
      expect(result).toBe('010fff')
    })

    it('handles empty array', () => {
      const bytes = new Uint8Array([])
      const result = bytesToHex(bytes)
      expect(result).toBe('')
    })
  })

  describe('sign', () => {
    it('calls ed25519 signAsync with correct parameters', async () => {
      const { signAsync } = await import('@noble/ed25519')
      
      const message = new Uint8Array([1, 2, 3])
      const privateKey = new Uint8Array(32).fill(1)
      
      const result = await sign(message, privateKey)
      
      expect(signAsync).toHaveBeenCalledWith(message, privateKey)
      expect(result).toEqual(new Uint8Array(64).fill(3))
    })
  })

  describe('generateEd25519KeyPair', () => {
    it('generates keypair with private and public keys', async () => {
      const keyPair = await generateEd25519KeyPair()
      
      expect(keyPair).toHaveProperty('privateKey')
      expect(keyPair).toHaveProperty('publicKey')
      
      expect(keyPair.privateKey).toEqual(new Uint8Array(32).fill(1))
      expect(keyPair.publicKey).toEqual(new Uint8Array(32).fill(2))
      
      // Test that we can manually convert to base64
      const publicKeyBase64 = bytesToBase64(keyPair.publicKey)
      expect(publicKeyBase64).toBe('AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=')
    })

    it('throws error when key generation fails', async () => {
      const { utils } = await import('@noble/ed25519')
      utils.randomPrivateKey.mockImplementationOnce(() => {
        throw new Error('Key generation failed')
      })

      await expect(generateEd25519KeyPair()).rejects.toThrow('Failed to generate Ed25519 keypair: Key generation failed')
    })
  })

  describe('round-trip conversions', () => {
    it('bytes -> base64 -> bytes maintains data integrity', () => {
      const originalBytes = new Uint8Array([1, 2, 3, 4, 5, 255, 0, 128])
      const base64 = bytesToBase64(originalBytes)
      const roundTripBytes = base64ToBytes(base64)
      
      expect(roundTripBytes).toEqual(originalBytes)
    })

    it('bytes -> hex -> bytes maintains data integrity', () => {
      const originalBytes = new Uint8Array([1, 2, 3, 4, 5, 255, 0, 128])
      const hex = bytesToHex(originalBytes)
      const roundTripBytes = hexToBytes(hex)
      
      expect(roundTripBytes).toEqual(originalBytes)
    })
  })
})