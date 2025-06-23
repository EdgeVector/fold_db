// Ed25519 utility functions - kept minimal for only required functions

import * as ed from '@noble/ed25519';
import type { KeyPair } from '../types/cryptography';

/**
 * Generate a new Ed25519 keypair
 * @returns Promise<KeyPair> - Generated keypair
 */
export async function generateEd25519KeyPair(): Promise<KeyPair> {
  try {
    // Generate a secure random private key
    const privateKey = ed.utils.randomPrivateKey();
    
    // Derive the public key from the private key
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    
    return {
      privateKey,
      publicKey
    };
  } catch (error) {
    throw new Error(`Failed to generate Ed25519 keypair: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

/**
 * Convert a Uint8Array to a Base64 string.
 * @param bytes - The byte array to convert.
 * @returns The Base64-encoded string.
 */
export function bytesToBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes));
}

/**
 * Convert a Base64 string to a Uint8Array.
 * @param base64 - The Base64-encoded string to convert.
 * @returns The decoded byte array.
 */
export function base64ToBytes(base64: string): Uint8Array {
  const binString = atob(base64);
  const len = binString.length;
  const bytes = new Uint8Array(len);
  for (let i = 0; i < len; i++) {
    bytes[i] = binString.charCodeAt(i);
  }
  return bytes;
}

/**
 * Convert a hex string to a Uint8Array.
 * @param hex - The hex string to convert.
 * @returns The decoded byte array.
 */
export function hexToBytes(hex: string): Uint8Array {
  if (hex.length === 0) {
    return new Uint8Array([]);
  }
  
  if (hex.length % 2 !== 0) {
    throw new Error('Hex string must have an even length');
  }
  
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    const byte = parseInt(hex.substr(i, 2), 16);
    if (isNaN(byte)) {
      throw new Error(`Invalid hex character at position ${i}`);
    }
    bytes[i / 2] = byte;
  }
  return bytes;
}

/**
 * Convert a Uint8Array to a hex string.
 * @param bytes - The byte array to convert.
 * @returns The hex-encoded string.
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Sign a message using an Ed25519 private key.
 * @param message - The message to sign (Uint8Array).
 * @param privateKey - The private key to use for signing.
 * @returns Promise<Uint8Array> - The signature.
 */
export async function sign(
  message: Uint8Array,
  privateKey: Uint8Array
): Promise<Uint8Array> {
  try {
    return await ed.signAsync(message, privateKey);
  } catch (error) {
    throw new Error(`Failed to sign message: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

/**
 * Verify a signature using an Ed25519 public key.
 * @param signature - The signature to verify (Uint8Array).
 * @param message - The original message (Uint8Array).
 * @param publicKey - The public key to use for verification.
 * @returns Promise<boolean> - True if the signature is valid.
 */
export async function verify(
  signature: Uint8Array,
  message: Uint8Array,
  publicKey: Uint8Array
): Promise<boolean> {
  try {
    return await ed.verifyAsync(signature, message, publicKey);
  } catch (error) {
    throw new Error(`Failed to verify signature: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}

