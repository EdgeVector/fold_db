/* global Buffer, atob, btoa */
/**
 * @fileoverview Crypto Utilities
 *
 * Provides cryptographic utilities for signing and validation.
 * Used by Mutation components.
 *
 * @module cryptoUtils
 * @since 2.0.0
 */

import { utils, sign, verify } from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// Polyfill for browser environment
const decodeBase64 = (str) => {
  if (typeof Buffer !== "undefined") return Buffer.from(str, "base64");
  return Uint8Array.from(atob(str), (c) => c.charCodeAt(0));
};

const encodeBase64 = (bytes) => {
  if (typeof Buffer !== "undefined")
    return Buffer.from(bytes).toString("base64");
  const binString = Array.from(bytes, (x) => String.fromCharCode(x)).join("");
  return btoa(binString);
};

// Set up SHA-512 hash function for ed25519
utils.sha512Sync = (...m) => sha512(utils.concatBytes(...m));

/**
 * Sign a payload with a private key
 * @param {string|object} payload - The payload to sign
 * @param {string} privateKeyBase64 - Base64 encoded private key
 * @returns {Promise<string>} Base64 encoded signature
 */
export async function signPayload(payload, privateKeyBase64) {
  const payloadString =
    typeof payload === "string" ? payload : JSON.stringify(payload);
  const privateKey = decodeBase64(privateKeyBase64);
  const message = new TextEncoder().encode(payloadString);

  const signature = await sign(message, privateKey);
  return encodeBase64(signature);
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
    const payloadString =
      typeof payload === "string" ? payload : JSON.stringify(payload);
    const sig = decodeBase64(signature);
    const publicKey = decodeBase64(publicKeyBase64);
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
    return { isValid: false, error: "Range key is required" };
  }

  if (!schema?.isRange) {
    return { isValid: false, error: "Schema is not a range schema" };
  }

  // Basic validation - in real implementation this would be more thorough
  if (rangeKey.length < 10) {
    return { isValid: false, error: "Range key too short" };
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
  return encodeBase64(array).substring(0, length);
}

/**
 * Convert base64 string to bytes
 * @param {string} base64 - Base64 encoded string
 * @returns {Uint8Array} Decoded bytes
 */
export function base64ToBytes(base64) {
  return decodeBase64(base64);
}

/**
 * Convert bytes to base64 string
 * @param {Uint8Array} bytes - Bytes to encode
 * @returns {string} Base64 encoded string
 */
export function bytesToBase64(bytes) {
  return encodeBase64(bytes);
}

/** The constant key ID used by the Rust backend (must match SINGLE_PUBLIC_KEY_ID) */
export const SYSTEM_PUBLIC_KEY_ID = "SYSTEM_WIDE_PUBLIC_KEY";

/**
 * Create a SignedMessage envelope matching the Rust MessageSigner::sign_message() protocol.
 *
 * Signing format: sign(payload_bytes || timestamp_be_bytes(8) || key_id_bytes)
 *
 * @param {object} payload - The JSON payload to sign
 * @param {string} privateKeyBase64 - Base64 encoded Ed25519 private key (32 bytes)
 * @returns {Promise<object>} A SignedMessage object: { payload, public_key_id, signature, timestamp }
 */
export async function createSignedMessage(payload, privateKeyBase64) {
  // Serialize payload to canonical JSON bytes (matches Rust's serde_json::to_vec)
  const payloadBytes = new TextEncoder().encode(JSON.stringify(payload));

  // Unix timestamp in seconds
  const timestamp = Math.floor(Date.now() / 1000);

  // Build the message to sign: payload_bytes + timestamp(i64 big-endian) + key_id_bytes
  const timestampBytes = new ArrayBuffer(8);
  new DataView(timestampBytes).setBigInt64(0, BigInt(timestamp));
  const keyIdBytes = new TextEncoder().encode(SYSTEM_PUBLIC_KEY_ID);

  const messageToSign = new Uint8Array(
    payloadBytes.length + 8 + keyIdBytes.length,
  );
  messageToSign.set(payloadBytes, 0);
  messageToSign.set(new Uint8Array(timestampBytes), payloadBytes.length);
  messageToSign.set(keyIdBytes, payloadBytes.length + 8);

  // Sign with Ed25519 (ensure Uint8Array, not Buffer, for @noble/ed25519)
  const privateKey = Uint8Array.from(decodeBase64(privateKeyBase64));
  const signatureBytes = await sign(messageToSign, privateKey);

  return {
    payload: encodeBase64(payloadBytes),
    public_key_id: SYSTEM_PUBLIC_KEY_ID,
    signature: encodeBase64(signatureBytes),
    timestamp,
  };
}
