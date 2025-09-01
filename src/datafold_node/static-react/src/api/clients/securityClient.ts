/**
 * Security API Client - Unified Implementation
 * Replaces existing securityClient.ts with standardized approach
 * Handles authentication, key management, and cryptographic operations
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES, API_CACHE_TTL, CACHE_KEYS } from '../../constants/api';
import type { EnhancedApiResponse, SecurityApiClient } from '../core/types';
import type { 
  SignedMessage
} from '../../types/cryptography';
import type { 
  VerificationResponse
} from '../../types/api';

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

/**
 * Unified Security API Client Implementation
 */
export class UnifiedSecurityClient implements SecurityApiClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: true, // Cache public keys and verification results
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Verify a signed message
   * UNPROTECTED - No authentication required (verification endpoint)
   * 
   * @param signedMessage The signed message to verify
   * @returns Promise resolving to verification result
   */
  async verifyMessage(signedMessage: SignedMessage): Promise<EnhancedApiResponse<VerificationResponse>> {
    return this.client.post<VerificationResponse>(
      API_ENDPOINTS.VERIFY_MESSAGE,
      signedMessage,
      {
        requiresAuth: false, // Verification is public
        timeout: API_TIMEOUTS.CRYPTO_OPERATIONS, // Reasonable timeout for crypto operations
        retries: API_RETRIES.STANDARD, // Allow retries for network issues
        cacheable: true, // Cache verification results
        cacheTtl: API_CACHE_TTL.VERIFICATION_RESULTS, // Cache for 5 minutes
        cacheKey: `${CACHE_KEYS.VERIFY}:${signedMessage.signature}:${signedMessage.public_key_id}`
      }
    );
  }

  /**
   * Get the system's public key
   * UNPROTECTED - No authentication required (public key is public)
   * 
   * @returns Promise resolving to system public key
   */
  async getSystemPublicKey(): Promise<EnhancedApiResponse<SystemKeyResponse>> {
    return this.client.get<SystemKeyResponse>(
      API_ENDPOINTS.GET_SYSTEM_PUBLIC_KEY,
      {
        requiresAuth: false, // System public key is public
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.CRITICAL, // Multiple retries for critical system data
        cacheable: true, // Cache system public key
        cacheTtl: API_CACHE_TTL.SYSTEM_PUBLIC_KEY, // Cache for 1 hour (system key doesn't change often)
        cacheKey: CACHE_KEYS.SYSTEM_PUBLIC_KEY
      }
    );
  }

  /**
   * Validate a public key's format and cryptographic properties
   * This is a client-side validation helper
   * 
   * @param publicKey The public key to validate (base64 encoded)
   * @returns Validation result with details
   */
  validatePublicKeyFormat(publicKey: string): {
    isValid: boolean;
    format?: string;
    length?: number;
    error?: string;
  } {
    try {
      // Basic validation for Ed25519 public keys
      if (!publicKey || typeof publicKey !== 'string') {
        return {
          isValid: false,
          error: 'Public key must be a non-empty string'
        };
      }

      // Remove any whitespace
      const cleanKey = publicKey.trim();

      // Check if it's valid base64
      try {
        const decoded = atob(cleanKey);
        const length = decoded.length;

        // Ed25519 public keys are 32 bytes
        if (length === 32) {
          return {
            isValid: true,
            format: 'Ed25519',
            length: length
          };
        } else {
          return {
            isValid: false,
            format: 'Unknown',
            length: length,
            error: `Invalid key length: ${length} bytes (expected 32 for Ed25519)`
          };
        }
      } catch {
        return {
          isValid: false,
          error: 'Invalid base64 encoding'
        };
      }
    } catch (error) {
      return {
        isValid: false,
        error: `Validation error: ${error.message}`
      };
    }
  }

  /**
   * Get security status and configuration
   * PROTECTED - Requires authentication
   * 
   * @returns Promise resolving to security status
   */
  async getSecurityStatus(): Promise<EnhancedApiResponse<SecurityStatus>> {
    return this.client.get<SecurityStatus>(
      '/api/security/status',
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.STANDARD,
        cacheable: true,
        cacheTtl: API_CACHE_TTL.SECURITY_STATUS, // Cache for 1 minute
        cacheKey: CACHE_KEYS.SECURITY_STATUS
      }
    );
  }

  /**
   * Validate a signed message's structure before sending for verification
   * This is a client-side validation helper
   * 
   * @param signedMessage The signed message to validate
   * @returns Validation result
   */
  validateSignedMessage(signedMessage: SignedMessage): {
    isValid: boolean;
    errors: string[];
  } {
    const errors: string[] = [];

    if (!signedMessage || typeof signedMessage !== 'object') {
      errors.push('Signed message must be an object');
      return { isValid: false, errors };
    }

    // Validate payload
    if (!signedMessage.payload || typeof signedMessage.payload !== 'string') {
      errors.push('Payload must be a non-empty base64 string');
    }

    // Validate signature
    if (!signedMessage.signature || typeof signedMessage.signature !== 'string') {
      errors.push('Signature must be a non-empty base64 string');
    }

    // Validate public key ID
    if (!signedMessage.public_key_id || typeof signedMessage.public_key_id !== 'string') {
      errors.push('Public key ID must be a non-empty string');
    }

    // Validate timestamp
    if (!signedMessage.timestamp || typeof signedMessage.timestamp !== 'number') {
      errors.push('Timestamp must be a Unix timestamp number');
    } else {
      const now = Math.floor(Date.now() / 1000);
      const messageAge = now - signedMessage.timestamp;
      
      // Check if message is too old (5 minutes)
      if (messageAge > 300) {
        errors.push('Message is too old (timestamp more than 5 minutes ago)');
      }
      
      // Check if message is from the future (allow 1 minute skew)
      if (messageAge < -60) {
        errors.push('Message timestamp is too far in the future');
      }
    }

    // Validate nonce (optional)
    if (signedMessage.nonce && typeof signedMessage.nonce !== 'string') {
      errors.push('Nonce must be a string if provided');
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  }

  /**
   * Get API metrics for security operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(metric => 
      metric.url.includes('/security')
    );
  }

  /**
   * Clear security-related cache
   */
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const securityClient = new UnifiedSecurityClient();

// Export factory function for custom instances
export function createSecurityClient(client?: ApiClient): UnifiedSecurityClient {
  return new UnifiedSecurityClient(client);
}

// Backward compatibility exports - these will be deprecated
export const verifyMessage = securityClient.verifyMessage.bind(securityClient);
export const getSystemPublicKey = securityClient.getSystemPublicKey.bind(securityClient);

// New exports
export const validatePublicKeyFormat = securityClient.validatePublicKeyFormat.bind(securityClient);
export const validateSignedMessage = securityClient.validateSignedMessage.bind(securityClient);
export const getSecurityStatus = securityClient.getSecurityStatus.bind(securityClient);

export default securityClient;