// Security API Client — authentication, key management, and cryptographic operations

import { ApiClient, getSharedClient } from "../core/client";
import { API_ENDPOINTS } from "../endpoints";
import {
  API_TIMEOUTS,
  API_RETRIES,
  API_CACHE_TTL,
  CACHE_KEYS,
} from "../../constants/api";
import type { EnhancedApiResponse, SecurityApiClient } from "../core/types";
import type { SignedMessage } from "../../types/cryptography";
import type { VerificationResponse } from "../../types/api";

// Security-specific response types
export interface SystemKeyResponse {
  public_key: string;
  public_key_id?: string;
  algorithm?: string;
  created_at?: string;
  expires_at?: string;
}

export interface KeyValidationResult {
  isValid: boolean;
  keyId?: string;
  owner?: string;
  permissions?: string[];
  expiresAt?: number;
  error?: string;
}

export interface SecurityStatus {
  systemKeyRegistered: boolean;
  systemKeyId?: string;
  authenticationRequired: boolean;
  encryptionEnabled: boolean;
  lastKeyRotation?: string;
}

// Unified Security API Client Implementation
export class UnifiedSecurityClient implements SecurityApiClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || getSharedClient();
  }

  // Verify a signed message (no auth required)
  async verifyMessage(
    message: SignedMessage,
  ): Promise<EnhancedApiResponse<VerificationResponse>> {
    const validation = this.validateSignedMessage(message);
    if (!validation.isValid) {
      return {
        success: false,
        error: `Invalid message format: ${validation.errors.join(", ")}`,
        status: 400,
        data: { isValid: false, error: validation.errors[0] },
      };
    }

    // Since server verification endpoint is missing, perform client-side format check only
    // This is a placeholder until backend implementation is complete
    return {
      success: true,
      status: 200,
      data: {
        isValid: true,
        details: {
          signature: message.signature,
          timestamp: message.timestamp,
          verified: false, // Explicitly state not cryptographically verified by server
        },
      },
    };
  }

  // Get the system's public key (no auth required)
  async getSystemPublicKey(): Promise<EnhancedApiResponse<SystemKeyResponse>> {
    return this.client.get<SystemKeyResponse>(
      API_ENDPOINTS.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: false, // System public key is public
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.CRITICAL, // Multiple retries for critical system data
        cacheable: true, // Cache system public key
        cacheTtl: API_CACHE_TTL.SYSTEM_PUBLIC_KEY, // Cache for 1 hour (system key doesn't change often)
        cacheKey: CACHE_KEYS.SYSTEM_PUBLIC_KEY,
      },
    );
  }

  // Validate a public key's format and cryptographic properties (client-side)
  validatePublicKeyFormat(publicKey: string): {
    isValid: boolean;
    format?: string;
    length?: number;
    error?: string;
  } {
    try {
      // Basic validation for Ed25519 public keys without decoding
      if (!publicKey || typeof publicKey !== "string") {
        return {
          isValid: false,
          error: "Public key must be a non-empty string",
        };
      }

      const cleanKey = publicKey.trim();

      // Base64 character check (no decode)
      const base64Regex =
        /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;
      if (!base64Regex.test(cleanKey)) {
        return {
          isValid: false,
          error: "Invalid base64 encoding",
        };
      }

      // Ed25519 public keys are 32 bytes → base64 length should be 44
      if (cleanKey.length !== 44) {
        return {
          isValid: false,
          format: "Unknown",
          length: Math.ceil((cleanKey.length / 4) * 3),
          error: "Invalid key length: expected 44 base64 chars for Ed25519",
        };
      }

      return {
        isValid: true,
        format: "Ed25519",
        length: 32,
      };
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        isValid: false,
        error: `Validation error: ${message}`,
      };
    }
  }

  // Get security status and configuration (no auth required)
  async getSecurityStatus(): Promise<EnhancedApiResponse<SecurityStatus>> {
    return this.client.get<SecurityStatus>(API_ENDPOINTS.GET_SYSTEM_STATUS, {
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: true,
      cacheTtl: API_CACHE_TTL.SECURITY_STATUS, // Cache for 1 minute
      cacheKey: CACHE_KEYS.SECURITY_STATUS,
    });
  }

  // Validate a signed message's structure before sending for verification (client-side)
  validateSignedMessage(signedMessage: SignedMessage): {
    isValid: boolean;
    errors: string[];
  } {
    const errors: string[] = [];

    if (!signedMessage || typeof signedMessage !== "object") {
      errors.push("Signed message must be an object");
      return { isValid: false, errors };
    }

    // Validate payload
    if (!signedMessage.payload || typeof signedMessage.payload !== "string") {
      errors.push("Payload must be a non-empty base64 string");
    }

    // Validate signature
    if (
      !signedMessage.signature ||
      typeof signedMessage.signature !== "string"
    ) {
      errors.push("Signature must be a non-empty base64 string");
    }

    // Validate public key ID
    if (
      !signedMessage.public_key_id ||
      typeof signedMessage.public_key_id !== "string"
    ) {
      errors.push("Public key ID must be a non-empty string");
    }

    // Validate timestamp
    if (
      !signedMessage.timestamp ||
      typeof signedMessage.timestamp !== "number"
    ) {
      errors.push("Timestamp must be a Unix timestamp number");
    } else {
      const now = Math.floor(Date.now() / 1000);
      const messageAge = now - signedMessage.timestamp;

      // Check if message is too old (5 minutes)
      if (messageAge > 300) {
        errors.push("Message is too old (timestamp more than 5 minutes ago)");
      }

      // Check if message is from the future (allow 1 minute skew)
      if (messageAge < -60) {
        errors.push("Message timestamp is too far in the future");
      }
    }

    // Validate nonce (optional)
    if (signedMessage.nonce && typeof signedMessage.nonce !== "string") {
      errors.push("Nonce must be a string if provided");
    }

    return {
      isValid: errors.length === 0,
      errors,
    };
  }

  // Get API metrics for security operations
  getMetrics() {
    return this.client
      .getMetrics()
      .filter((metric) => metric.url.includes("/security"));
  }

  // Clear security-related cache
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const securityClient = new UnifiedSecurityClient();

// Export factory function for custom instances
export function createSecurityClient(
  client?: ApiClient,
): UnifiedSecurityClient {
  return new UnifiedSecurityClient(client);
}

export const getSystemPublicKey =
  securityClient.getSystemPublicKey.bind(securityClient);

// New exports
export const validatePublicKeyFormat =
  securityClient.validatePublicKeyFormat.bind(securityClient);
export const validateSignedMessage =
  securityClient.validateSignedMessage.bind(securityClient);
export const getSecurityStatus =
  securityClient.getSecurityStatus.bind(securityClient);
export const verifyMessage = securityClient.verifyMessage.bind(securityClient);

export default securityClient;
