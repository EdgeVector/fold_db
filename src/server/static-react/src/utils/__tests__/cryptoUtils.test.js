import { describe, it, expect } from "vitest";
import {
  createSignedMessage,
  SYSTEM_PUBLIC_KEY_ID,
  base64ToBytes,
  bytesToBase64,
} from "../cryptoUtils";
import * as ed from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha512";

// Set up SHA-512 hash function for ed25519 (both sync and async paths)
ed.etc.sha512Sync = (...m) => sha512(ed.etc.concatBytes(...m));
ed.etc.sha512Async = async (...m) => sha512(ed.etc.concatBytes(...m));

describe("createSignedMessage", () => {
  it("produces a valid SignedMessage envelope", async () => {
    // Generate a test keypair
    const privateKey = ed.utils.randomPrivateKey();
    const privateKeyBase64 = bytesToBase64(privateKey);

    const payload = { schema_name: "Test", data: "hello" };
    const signed = await createSignedMessage(payload, privateKeyBase64);

    // Check structure
    expect(signed).toHaveProperty("payload");
    expect(signed).toHaveProperty("public_key_id", SYSTEM_PUBLIC_KEY_ID);
    expect(signed).toHaveProperty("signature");
    expect(signed).toHaveProperty("timestamp");
    expect(typeof signed.timestamp).toBe("number");

    // Verify payload is base64-encoded JSON of the original
    const decodedPayload = JSON.parse(
      new TextDecoder().decode(base64ToBytes(signed.payload)),
    );
    expect(decodedPayload).toEqual(payload);
  });

  it("signature verifies against the matching public key", async () => {
    const privateKey = ed.utils.randomPrivateKey();
    const publicKey = await ed.getPublicKeyAsync(privateKey);
    const privateKeyBase64 = bytesToBase64(privateKey);

    const payload = { test: "verification" };
    const signed = await createSignedMessage(payload, privateKeyBase64);

    // Reconstruct the message that was signed (matching Rust's format)
    const payloadBytes = base64ToBytes(signed.payload);
    const timestampBytes = new ArrayBuffer(8);
    new DataView(timestampBytes).setBigInt64(0, BigInt(signed.timestamp));
    const keyIdBytes = new TextEncoder().encode(signed.public_key_id);

    const messageToVerify = new Uint8Array(
      payloadBytes.length + 8 + keyIdBytes.length,
    );
    messageToVerify.set(payloadBytes, 0);
    messageToVerify.set(new Uint8Array(timestampBytes), payloadBytes.length);
    messageToVerify.set(keyIdBytes, payloadBytes.length + 8);

    const signatureBytes = Uint8Array.from(base64ToBytes(signed.signature));
    const isValid = await ed.verifyAsync(
      signatureBytes,
      messageToVerify,
      publicKey,
    );
    expect(isValid).toBe(true);
  });

  it("signature fails with wrong public key", async () => {
    const privateKey1 = ed.utils.randomPrivateKey();
    const privateKey2 = ed.utils.randomPrivateKey();
    const wrongPublicKey = await ed.getPublicKeyAsync(privateKey2);
    const privateKeyBase64 = bytesToBase64(privateKey1);

    const payload = { test: "wrong_key" };
    const signed = await createSignedMessage(payload, privateKeyBase64);

    // Reconstruct signed message
    const payloadBytes = Uint8Array.from(base64ToBytes(signed.payload));
    const timestampBytes = new ArrayBuffer(8);
    new DataView(timestampBytes).setBigInt64(0, BigInt(signed.timestamp));
    const keyIdBytes = new TextEncoder().encode(signed.public_key_id);

    const messageToVerify = new Uint8Array(
      payloadBytes.length + 8 + keyIdBytes.length,
    );
    messageToVerify.set(payloadBytes, 0);
    messageToVerify.set(new Uint8Array(timestampBytes), payloadBytes.length);
    messageToVerify.set(keyIdBytes, payloadBytes.length + 8);

    const signatureBytes = Uint8Array.from(base64ToBytes(signed.signature));
    const isValid = await ed.verifyAsync(
      signatureBytes,
      messageToVerify,
      wrongPublicKey,
    );
    expect(isValid).toBe(false);
  });

  it("uses SYSTEM_WIDE_PUBLIC_KEY as the key ID", async () => {
    const privateKey = ed.utils.randomPrivateKey();
    const privateKeyBase64 = bytesToBase64(privateKey);

    const signed = await createSignedMessage({ x: 1 }, privateKeyBase64);
    expect(signed.public_key_id).toBe("SYSTEM_WIDE_PUBLIC_KEY");
  });

  it("timestamp is a recent Unix timestamp in seconds", async () => {
    const privateKey = ed.utils.randomPrivateKey();
    const privateKeyBase64 = bytesToBase64(privateKey);

    const before = Math.floor(Date.now() / 1000);
    const signed = await createSignedMessage({ x: 1 }, privateKeyBase64);
    const after = Math.floor(Date.now() / 1000);

    expect(signed.timestamp).toBeGreaterThanOrEqual(before);
    expect(signed.timestamp).toBeLessThanOrEqual(after);
  });
});
