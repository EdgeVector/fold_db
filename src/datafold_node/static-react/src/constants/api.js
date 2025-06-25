/**
 * API Configuration Constants
 * Centralized API configuration per Section 2.1.12 requirements
 */

// Request Configuration
export const API_REQUEST_TIMEOUT_MS = 30000;
export const API_RETRY_ATTEMPTS = 3;
export const API_RETRY_DELAY_MS = 1000;
export const API_BATCH_REQUEST_LIMIT = 20;

// HTTP Status Codes
export const HTTP_STATUS_CODES = {
  OK: 200,
  CREATED: 201,
  ACCEPTED: 202,
  NO_CONTENT: 204,
  BAD_REQUEST: 400,
  UNAUTHORIZED: 401,
  FORBIDDEN: 403,
  NOT_FOUND: 404,
  CONFLICT: 409,
  INTERNAL_SERVER_ERROR: 500,
  BAD_GATEWAY: 502,
  SERVICE_UNAVAILABLE: 503,
  GATEWAY_TIMEOUT: 504
};

// Content Types
export const CONTENT_TYPES = {
  JSON: 'application/json',
  FORM_DATA: 'multipart/form-data',
  URL_ENCODED: 'application/x-www-form-urlencoded',
  TEXT: 'text/plain'
};

// Request Headers
export const REQUEST_HEADERS = {
  CONTENT_TYPE: 'Content-Type',
  AUTHORIZATION: 'Authorization',
  SIGNED_REQUEST: 'X-Signed-Request',
  REQUEST_ID: 'X-Request-ID'
};

// Error Messages
export const ERROR_MESSAGES = {
  NETWORK_ERROR: 'Network connection failed. Please check your internet connection.',
  TIMEOUT_ERROR: 'Request timed out. Please try again.',
  AUTHENTICATION_ERROR: 'Authentication required. Please ensure you are properly authenticated.',
  SCHEMA_STATE_ERROR: 'Schema operation not allowed. Only approved schemas can be accessed.',
  SERVER_ERROR: 'Server error occurred. Please try again later.',
  VALIDATION_ERROR: 'Request validation failed. Please check your input.',
  NOT_FOUND_ERROR: 'Requested resource not found.',
  PERMISSION_ERROR: 'Permission denied. You do not have access to this resource.',
  RATE_LIMIT_ERROR: 'Too many requests. Please wait before trying again.'
};

// Cache Configuration
export const CACHE_CONFIG = {
  DEFAULT_TTL_MS: 300000, // 5 minutes
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: 600000, // 10 minutes
  SYSTEM_STATUS_CACHE_TTL_MS: 30000 // 30 seconds
};

// Retry Configuration
export const RETRY_CONFIG = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 10000
};

// API Base Configuration
export const API_CONFIG = {
  BASE_URL: '/api',
  VERSION: 'v1',
  DEFAULT_TIMEOUT: API_REQUEST_TIMEOUT_MS,
  DEFAULT_RETRIES: API_RETRY_ATTEMPTS
};

// Schema State Constants (SCHEMA-002 compliance)
export const SCHEMA_STATES = {
  AVAILABLE: 'available',
  APPROVED: 'approved',
  BLOCKED: 'blocked'
};

// Schema Operation Types
export const SCHEMA_OPERATIONS = {
  READ: 'read',
  WRITE: 'write',
  APPROVE: 'approve',
  BLOCK: 'block',
  MUTATION: 'mutation',
  QUERY: 'query'
};

export default {
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
};