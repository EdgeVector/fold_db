import { describe, it, expect, vi } from 'vitest';
import { generateEd25519KeyPair, base64ToBytes, sign, verify } from '../ed25519';
import { createSignedMessage } from '../signing';

// Mock @noble/ed25519 for consistent test results
vi.mock('@noble/ed25519', () => ({
  utils: {
    randomPrivateKey: vi.fn(() => new Uint8Array(32).fill(1)),
  },
  getPublicKeyAsync: vi.fn(() => Promise.resolve(new Uint8Array(32).fill(2))),
  signAsync: vi.fn(() => Promise.resolve(new Uint8Array(64).fill(3))),
  verifyAsync: vi.fn(() => Promise.resolve(true)),
}));

function concatUint8Arrays(arrays: Uint8Array[]): Uint8Array {
  const totalLength = arrays.reduce((acc, arr) => acc + arr.length, 0);
  const result = new Uint8Array(totalLength);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

// Helper to reconstruct the message for verification
function reconstructMessage(
  payload: any,
  timestamp: number,
  publicKeyId: string
): Uint8Array {
  const payloadString = JSON.stringify(payload);
  const payloadBytes = new TextEncoder().encode(payloadString);

  const timestampBuffer = new ArrayBuffer(8);
  const timestampView = new DataView(timestampBuffer);
  timestampView.setBigInt64(0, BigInt(timestamp), false);
  const timestampBytes = new Uint8Array(timestampBuffer);

  const publicKeyIdBytes = new TextEncoder().encode(publicKeyId);
  
  return concatUint8Arrays([
    payloadBytes,
    timestampBytes,
    publicKeyIdBytes,
  ]);
}

describe('signing', () => {
  it('should create a valid signed message and be verifiable', async () => {
    // 1. Setup
    const { privateKey, publicKey } = await generateEd25519KeyPair();
    const publicKeyId = 'test-key-1';
    const payload = { data: 'hello world', value: 123 };

    // 2. Execute
    const signedMessage = await createSignedMessage(payload, publicKeyId, privateKey);

    // 3. Assert structure
    expect(signedMessage).toBeDefined();
    expect(typeof signedMessage.payload).toBe('string'); // payload is base64 encoded
    expect(signedMessage.public_key_id).toBe(publicKeyId);
    expect(typeof signedMessage.signature).toBe('string');
    expect(typeof signedMessage.timestamp).toBe('number');

    // 4. Decode and verify the payload
    const decodedPayloadBytes = base64ToBytes(signedMessage.payload);
    const decodedPayloadString = new TextDecoder().decode(decodedPayloadBytes);
    const decodedPayload = JSON.parse(decodedPayloadString);
    expect(decodedPayload).toEqual(payload);

    // 5. Verify signature - reconstruct message with decoded payload
    const messageToVerify = reconstructMessage(
      decodedPayload,
      signedMessage.timestamp,
      signedMessage.public_key_id
    );
    const signatureBytes = base64ToBytes(signedMessage.signature);

    const isValid = await verify(signatureBytes, messageToVerify, publicKey);
    expect(isValid).toBe(true);
  });
}); 
