/**
 * Unified API Client - Main Export Index
 * Central export point for all API clients and utilities
 * TASK-004: API Client Standardization and Unification
 */

// ============================================================================
// CORE API CLIENT EXPORTS
// ============================================================================

export { 
  ApiClient, 
  createApiClient, 
  defaultApiClient 
} from './core/client';

export type {
  ApiResponse,
  EnhancedApiResponse,
  RequestOptions,
  ApiClientConfig,
  RequestMetrics,
  BatchRequest,
  BatchResponse,
  ApiClientInstance
} from './core/types';

export {
  ApiError,
  AuthenticationError,
  SchemaStateError,
  NetworkError,
  TimeoutError,
  ValidationError,
  RateLimitError,
  ErrorFactory,
  isApiError,
  isNetworkError,
  isTimeoutError,
  isAuthenticationError,
  isSchemaStateError,
  isRetryableError,
  isValidationError,
  isRateLimitError
} from './core/errors';

// ============================================================================
// SPECIALIZED CLIENT EXPORTS
// ============================================================================

export {
  UnifiedSchemaClient,
  createSchemaClient,
  schemaClient,
  getSchemasByState,
  getAllSchemasWithState,
  getSchemaStatus,
  getSchema,
  approveSchema,
  blockSchema,
  getApprovedSchemas,
} from './clients/schemaClient';

export {
  UnifiedMutationClient,
  createMutationClient,
  mutationClient,
  MutationClient,
  executeMutation,
  executeQuery,
  validateSchemaForMutation
} from './clients/mutationClient';

export {
  UnifiedSecurityClient,
  createSecurityClient,
  securityClient,
  verifyMessage,
  getSystemPublicKey,
  validatePublicKeyFormat,
  validateSignedMessage,
  getSecurityStatus
} from './clients/securityClient';

// ============================================================================
// CONSTANTS
// ============================================================================

export {
  API_REQUEST_TIMEOUT_MS,
  API_RETRY_ATTEMPTS,
  API_RETRY_DELAY_MS,
  API_BATCH_REQUEST_LIMIT,
  HTTP_STATUS_CODES,
  CONTENT_TYPES,
  REQUEST_HEADERS,
  ERROR_MESSAGES,
  CACHE_CONFIG,
  RETRY_CONFIG,
  API_CONFIG,
  SCHEMA_STATES,
  SCHEMA_OPERATIONS
} from '../constants/api';

export { API_ENDPOINTS } from './endpoints';

// ============================================================================
// TYPE EXPORTS
// ============================================================================

export type { Schema } from '../types/schema';
export type { SignedMessage } from '../types/cryptography';
export type { VerificationResponse } from '../types/api';

// ============================================================================
// CONVENIENCE EXPORTS
// ============================================================================

import type { ApiClientConfig } from './core/types';
import {
  createApiClient,
  defaultApiClient
} from './core/client';
import {
  createSchemaClient,
  schemaClient
} from './clients/schemaClient';
import {
  createMutationClient,
  mutationClient
} from './clients/mutationClient';
import {
  createSecurityClient,
  securityClient
} from './clients/securityClient';
import { isRetryableError, isApiError } from './core/errors';

// Unified API suite factory function
export function createUnifiedApiSuite(config?: ApiClientConfig) {
  const baseClient = createApiClient(config);
  
  return {
    base: baseClient,
    schema: createSchemaClient(baseClient),
    mutation: createMutationClient(baseClient),
    security: createSecurityClient(baseClient)
  };
}

// Default unified API suite
export const defaultApiSuite = createUnifiedApiSuite({
  enableCache: true,
  enableLogging: true,
  enableMetrics: true
});

// Utility functions
export const ApiUtils = {
  isRetryableError,
  getUserErrorMessage: (error: unknown): string => {
    if (isApiError(error)) {
      return error.toUserMessage();
    }
    if (error instanceof Error) {
      return error.message;
    }
    return 'An unexpected error occurred';
  },
  generateRequestId: (): string => {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
  client: defaultApiClient,
  schema: schemaClient,
  mutation: mutationClient,
  security: securityClient,
  suite: defaultApiSuite,
  utils: ApiUtils
};