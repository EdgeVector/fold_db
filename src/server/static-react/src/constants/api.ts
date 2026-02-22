/**
 * API Configuration Constants - TypeScript Version
 * Centralized API configuration for API-STD-1 compliance
 * Extracted from repeated patterns across API clients for DRY enforcement
 */
import { API_BASE_URLS as GENERATED_API_BASE_URLS } from "../api/endpoints";

// Base Request Configuration
export const API_REQUEST_TIMEOUT_MS = 30000;
export const API_RETRY_ATTEMPTS = 3;
export const API_RETRY_DELAY_MS = 1000;
export const API_BATCH_REQUEST_LIMIT = 20;

// Operation-Specific Timeout Values
export const API_TIMEOUTS = {
  // Standard operations
  QUICK: 5000, // System status, basic gets
  STANDARD: 8000, // Schema reads, transforms, logs
  CONFIG: 10000, // Config changes, state changes, load/unload
  MUTATION: 15000, // Mutations, parameterized queries, AI validation
  BATCH: 30000, // Batch operations, database reset
  AI_PROCESSING: 120000, // Extended AI processing operations
  FOLDER_SCAN: 300000, // Smart folder scan (multiple sequential LLM calls)

  // Legacy support
  DEFAULT: 8000,
  CRYPTO_OPERATIONS: 8000,
  STATE_CHANGES: 10000,
  DESTRUCTIVE_OPERATIONS: 30000,
} as const;

// Operation-Specific Retry Configuration
export const API_RETRIES = {
  NONE: 0, // Mutations, destructive operations
  LIMITED: 1, // State changes, config operations, registrations
  STANDARD: 2, // Most read operations, network issues
  CRITICAL: 3, // System status, critical system data

  // Legacy support
  DEFAULT: 2,
  STATE_CHANGING: 1,
  NETWORK_RESILIENT: 2,
  SYSTEM_CRITICAL: 3,
} as const;

// Cache TTL Configuration
export const API_CACHE_TTL = {
  IMMEDIATE: 30000, // 30 seconds - system status
  SHORT: 60000, // 1 minute - queries, schema status
  MEDIUM: 180000, // 3 minutes - schema state, transforms
  STANDARD: 300000, // 5 minutes - schemas, mutation history
  LONG: 3600000, // 1 hour - system public key

  // Semantic aliases
  SYSTEM_STATUS: 30000,
  QUERY_RESULTS: 60000,
  PARAMETERIZED_QUERIES: 120000,
  SCHEMA_STATE: 180000,
  SCHEMA_DATA: 300000,
  VERIFICATION_RESULTS: 300000,
  SECURITY_STATUS: 60000,
  SYSTEM_PUBLIC_KEY: 3600000,
  TRANSFORM_DATA: 180000,
  INDIVIDUAL_TRANSFORMS: 300000,
  MUTATION_HISTORY: 300000,
} as const;

// API Base URLs (generated from Rust OpenAPI via endpoints.ts)
export const API_BASE_URLS = GENERATED_API_BASE_URLS;

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
  GATEWAY_TIMEOUT: 504,
} as const;

// Content Types
export const CONTENT_TYPES = {
  JSON: "application/json",
  FORM_DATA: "multipart/form-data",
  URL_ENCODED: "application/x-www-form-urlencoded",
  TEXT: "text/plain",
} as const;

// Request Headers
export const REQUEST_HEADERS = {
  CONTENT_TYPE: "Content-Type",
  AUTHORIZATION: "Authorization",
  SIGNED_REQUEST: "X-Signed-Request",
  REQUEST_ID: "X-Request-ID",
  AUTHENTICATED: "X-Authenticated",
} as const;

// Error Messages
export const ERROR_MESSAGES = {
  NETWORK_ERROR:
    "Network connection failed. Please check your internet connection.",
  TIMEOUT_ERROR: "Request timed out. Please try again.",
  AUTHENTICATION_ERROR:
    "Authentication required. Please ensure you are properly authenticated.",
  SCHEMA_STATE_ERROR:
    "Schema operation not allowed. Only approved schemas can be accessed.",
  SERVER_ERROR: "Server error occurred. Please try again later.",
  VALIDATION_ERROR: "Request validation failed. Please check your input.",
  NOT_FOUND_ERROR: "Requested resource not found.",
  PERMISSION_ERROR:
    "Permission denied. You do not have access to this resource.",
  RATE_LIMIT_ERROR: "Too many requests. Please wait before trying again.",
} as const;

// Cache Configuration
export const CACHE_CONFIG = {
  DEFAULT_TTL_MS: API_CACHE_TTL.STANDARD,
  MAX_CACHE_SIZE: 100,
  SCHEMA_CACHE_TTL_MS: API_CACHE_TTL.SCHEMA_DATA,
  SYSTEM_STATUS_CACHE_TTL_MS: API_CACHE_TTL.SYSTEM_STATUS,
} as const;

// Retry Configuration
export const RETRY_CONFIG = {
  RETRYABLE_STATUS_CODES: [408, 429, 500, 502, 503, 504],
  EXPONENTIAL_BACKOFF_MULTIPLIER: 2,
  MAX_RETRY_DELAY_MS: 10000,
} as const;

// API Base Configuration
export const API_CONFIG = {
  // Use relative path for CloudFront compatibility
  BASE_URL: "/api",
  VERSION: "v1",
  DEFAULT_TIMEOUT: API_TIMEOUTS.STANDARD,
  DEFAULT_RETRIES: API_RETRIES.STANDARD,
} as const;

// Schema State Constants - re-exported from canonical source (schemas.js)
import { SCHEMA_STATES } from "./schemas";
export { SCHEMA_STATES };

// Schema Operation Types
export const SCHEMA_OPERATIONS = {
  READ: "read",
  WRITE: "write",
  APPROVE: "approve",
  BLOCK: "block",
  MUTATION: "mutation",
  QUERY: "query",
} as const;

// Operation Type Classification
export const OPERATION_TYPES = {
  QUERY: "query",
  MUTATION: "mutation",
  SYSTEM: "system",
  INGESTION: "ingestion",
  SECURITY: "security",
  TRANSFORM: "transform",
} as const;

// Request Priority Levels
export const REQUEST_PRIORITIES = {
  LOW: "low",
  NORMAL: "normal",
  HIGH: "high",
  CRITICAL: "critical",
} as const;

// Cache Key Prefixes
export const CACHE_KEYS = {
  SCHEMAS: "schemas",
  SCHEMA: "schema",
  TRANSFORMS: "transforms",
  TRANSFORM: "transform",
  SYSTEM_STATUS: "system-status",
  SECURITY_STATUS: "security-status",
  SYSTEM_PUBLIC_KEY: "system-public-key",
  VERIFY: "verify",
  PARAMETERIZED_QUERY: "parameterized-query",
} as const;

// Default export for backwards compatibility
export default {
  API_REQUEST_TIMEOUT_MS,
  API_RETRY_ATTEMPTS,
  API_RETRY_DELAY_MS,
  API_BATCH_REQUEST_LIMIT,
  API_TIMEOUTS,
  API_RETRIES,
  API_CACHE_TTL,
  API_BASE_URLS,
  HTTP_STATUS_CODES,
  CONTENT_TYPES,
  REQUEST_HEADERS,
  ERROR_MESSAGES,
  CACHE_CONFIG,
  RETRY_CONFIG,
  API_CONFIG,
  SCHEMA_STATES,
  SCHEMA_OPERATIONS,
  OPERATION_TYPES,
  REQUEST_PRIORITIES,
  CACHE_KEYS,
};

// Type definitions for better type safety
export type SchemaState = (typeof SCHEMA_STATES)[keyof typeof SCHEMA_STATES];
export type SchemaOperation =
  (typeof SCHEMA_OPERATIONS)[keyof typeof SCHEMA_OPERATIONS];
export type OperationType =
  (typeof OPERATION_TYPES)[keyof typeof OPERATION_TYPES];
export type RequestPriority =
  (typeof REQUEST_PRIORITIES)[keyof typeof REQUEST_PRIORITIES];
export type HttpStatusCode =
  (typeof HTTP_STATUS_CODES)[keyof typeof HTTP_STATUS_CODES];
