// Types for Ed25519 key management

export interface KeyPair {
  privateKey: Uint8Array;
  publicKey: Uint8Array;
}

export interface SignedMessage {
  payload: string; // Base64-encoded JSON payload
  signature: string; // Base64-encoded signature
  public_key_id: string;
  timestamp: number; // UNIX timestamp in seconds
  nonce?: string;
}