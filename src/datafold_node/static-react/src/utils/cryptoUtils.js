/**
 * @fileoverview Crypto Utilities
 * 
 * Provides cryptographic utilities for signing and validation.
 * Used by Mutation components.
 * 
 * @module cryptoUtils
 * @since 2.0.0
 */

import { utils, sign, verify } from '@noble/ed25519';
import { sha512 } from '@noble/hashes/sha512';
import { Buffer } from 'buffer';

// Set up SHA-512 hash function for ed25519
utils.sha512Sync = (...m) => sha512(utils.concatBytes(...m));

/**
 * Sign a payload with a private key
 * @param {string|object} payload - The payload to sign
 * @param {string} privateKeyBase64 - Base64 encoded private key
 * @returns {Promise<string>} Base64 encoded signature
 */
export async function signPayload(payload, privateKeyBase64) {
  const payloadString = typeof payload === 'string' ? payload : JSON.stringify(payload);
  const privateKey = Buffer.from(privateKeyBase64, 'base64');
  const message = new TextEncoder().encode(payloadString);
  
  const signature = await sign(message, privateKey);
  return Buffer.from(signature).toString('base64');
}

/**
 * Verify a signature
 * @param {string} signature - Base64 encoded signature
 * @param {string|object} payload - The original payload
 * @param {string} publicKeyBase64 - Base64 encoded public key
 * @returns {Promise<boolean>}
 */
export async function verifySignature(signature, payload, publicKeyBase64) {
  try {
    const payloadString = typeof payload === 'string' ? payload : JSON.stringify(payload);
    const sig = Buffer.from(signature, 'base64');
    const publicKey = Buffer.from(publicKeyBase64, 'base64');
    const message = new TextEncoder().encode(payloadString);
    
    return await verify(sig, message, publicKey);
  } catch {
    return false;
  }
}

/**
 * Validate a range key for range schema operations
 * @param {string} rangeKey - The range key to validate
 * @param {object} schema - The schema definition
 * @returns {object} Validation result
 */
export function validateRangeKey(rangeKey, schema) {
  if (!rangeKey) {
    return { isValid: false, error: 'Range key is required' };
  }
  
  if (!schema?.isRange) {
    return { isValid: false, error: 'Schema is not a range schema' };
  }
  
  // Basic validation - in real implementation this would be more thorough
  if (rangeKey.length < 10) {
    return { isValid: false, error: 'Range key too short' };
  }
  
  return { isValid: true };
}

/**
 * Generate a secure random string
 * @param {number} length - Length of the string
 * @returns {string}
 */
export function generateSecureRandom(length = 32) {
  const array = new Uint8Array(length);
  crypto.getRandomValues(array);
  return Buffer.from(array).toString('base64').substring(0, length);
}

/**
 * Convert base64 string to bytes
 * @param {string} base64 - Base64 encoded string
 * @returns {Uint8Array} Decoded bytes
 */
export function base64ToBytes(base64) {
  return Buffer.from(base64, 'base64');
}

/**
 * Convert bytes to base64 string
 * @param {Uint8Array} bytes - Bytes to encode
 * @returns {string} Base64 encoded string
 */
export function bytesToBase64(bytes) {
  return Buffer.from(bytes).toString('base64');
}