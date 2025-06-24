import { sign, bytesToBase64, bytesToHex } from './ed25519';
import type { SignedMessage } from '../types/cryptography';

/**
 * Concatenates multiple Uint8Arrays into a single Uint8Array.
 * @param arrays - The arrays to concatenate.
 * @returns A new Uint8Array containing all the bytes.
 */
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

/**
 * Creates a signed message object that is compatible with the backend verifier.
 *
 * @param payload - The JSON payload to sign.
 * @param publicKeyId - The ID of the public key to use for verification.
 * @param privateKey - The private key for signing.
 * @returns A promise that resolves to a SignedMessage object.
 */
export async function createSignedMessage(
  payload: any,
  publicKeyId: string,
  privateKey: Uint8Array
): Promise<SignedMessage> {
  // 1. Get UNIX timestamp in seconds
  const timestamp = Math.floor(Date.now() / 1000);

  // 2. Serialize payload to compact JSON bytes
  const payloadString = JSON.stringify(payload);
  const payloadBytes = new TextEncoder().encode(payloadString);

  // 3. Create timestamp bytes (64-bit Big Endian)
  const timestampBuffer = new ArrayBuffer(8);
  const timestampView = new DataView(timestampBuffer);
  // Use BigInt for 64-bit integers.
  timestampView.setBigInt64(0, BigInt(timestamp), false); // false for Big Endian
  const timestampBytes = new Uint8Array(timestampBuffer);

  // 4. Get public key ID bytes
  const publicKeyIdBytes = new TextEncoder().encode(publicKeyId);

  // 5. Concatenate message parts
  const messageToSign = concatUint8Arrays([
    payloadBytes,
    timestampBytes,
    publicKeyIdBytes,
  ]);

  // 6. Sign the message
  const signatureBytes = await sign(messageToSign, privateKey);

  // 7. Base64-encode the signature (backend expects base64)
  const signature = bytesToBase64(signatureBytes);

  // 8. Base64-encode the payload string
  const payloadBase64 = bytesToBase64(payloadBytes);

  // 9. Construct the SignedMessage object
  return {
    payload: payloadBase64,
    signature,
    public_key_id: publicKeyId,
    timestamp,
  };
} 