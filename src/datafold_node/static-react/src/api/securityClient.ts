/**
 * Security API Client - Unified Implementation
 * Handles security operations like message verification and key management
 * Part of API-STD-1 TASK-002 implementation
 */

import { ApiClient, createApiClient } from './core/client';
import { API_ENDPOINTS } from './endpoints';
import { API_TIMEOUTS, API_RETRIES } from '../constants/api';
import type { EnhancedApiResponse } from './core/types';
import type { SignedMessage } from '../types/cryptography';
import type {
  ApiResponse,
  VerificationResponse,
  KeyRegistrationResponse,
} from '../types/api';
import type { KeyRegistrationRequest } from '../types/cryptography';

// Security-specific response types
export interface SystemPublicKeyResponse {
  public_key: string;
  public_key_id?: string;
}

/**
 * Security API Client Class
 * Provides methods for cryptographic operations and security management
 */
class SecurityApiClient {
  private readonly client: ApiClient;

  constructor() {
    this.client = createApiClient({
      enableCache: true, // Enable caching for public keys
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Verify a signed message
   */
  async verifyMessage(signedMessage: SignedMessage): Promise<EnhancedApiResponse<VerificationResponse>> {
    return this.client.post(API_ENDPOINTS.VERIFY_MESSAGE, signedMessage, {
      requiresAuth: false, // Message verification is public
      timeout: API_TIMEOUTS.CRYPTO_OPERATIONS,
      retries: API_RETRIES.STANDARD,
      cacheable: false // Don't cache verification results
    });
  }

  /**
   * Register a public key
   */
  async registerPublicKey(request: KeyRegistrationRequest): Promise<EnhancedApiResponse<KeyRegistrationResponse>> {
    return this.client.post(API_ENDPOINTS.REGISTER_PUBLIC_KEY, request, {
      requiresAuth: false, // System key registration doesn't require auth (initial setup)
      timeout: API_TIMEOUTS.CRYPTO_OPERATIONS,
      retries: API_RETRIES.LIMITED, // Limited retries for key operations
      cacheable: false // Don't cache registration results
    });
  }

  /**
   * Get the system public key
   */
  async getSystemPublicKey(): Promise<EnhancedApiResponse<SystemPublicKeyResponse>> {
    return this.client.get(API_ENDPOINTS.GET_SYSTEM_PUBLIC_KEY, {
      requiresAuth: false, // Public key is publicly accessible
      timeout: API_TIMEOUTS.STANDARD,
      retries: API_RETRIES.STANDARD,
      cacheable: true // Cache public keys for better performance
    });
  }
}

// Export singleton instance
export const securityClient = new SecurityApiClient();

// Export individual functions for backward compatibility
export async function verifyMessage(
  signedMessage: SignedMessage
): Promise<ApiResponse<VerificationResponse>> {
  const response = await securityClient.verifyMessage(signedMessage);
  return {
    success: response.success,
    data: response.data,
    error: response.error,
  };
}

export async function registerPublicKey(
  request: KeyRegistrationRequest,
): Promise<ApiResponse<KeyRegistrationResponse>> {
  const response = await securityClient.registerPublicKey(request);
  return {
    success: response.success,
    data: response.data,
    error: response.error,
  };
}

export async function getSystemPublicKey(): Promise<ApiResponse<{ public_key: string; public_key_id?: string }>> {
  const response = await securityClient.getSystemPublicKey();
  return {
    success: response.success,
    data: response.data,
    error: response.error,
  };
}
