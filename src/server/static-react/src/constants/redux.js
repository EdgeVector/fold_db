/**
 * Redux Constants for Schema State Management
 * TASK-003: State Management Consolidation with Redux
 *
 * This file contains all constants required for Redux schema state management
 * as per Section 2.1.12 of .cursorrules compliance requirements.
 */

// ============================================================================
// SCHEMA CACHE CONFIGURATION CONSTANTS
// ============================================================================

/**
 * Schema cache time-to-live in milliseconds (5 minutes)
 * Used for determining when to refetch schemas from the backend
 */
export const SCHEMA_CACHE_TTL_MS = 300000; // 5 minutes

/**
 * Number of retry attempts for schema fetch operations
 * Applied to network failures and transient errors
 */
export const SCHEMA_FETCH_RETRY_ATTEMPTS = 3;

/**
 * Timeout for individual schema operations in milliseconds
 * Applied to approve, block, and unload operations
 */
export const SCHEMA_OPERATION_TIMEOUT_MS = 10000;

/**
 * Maximum number of schemas to process in a single Redux batch
 * Used for performance optimization with large schema lists
 */
export const REDUX_BATCH_SIZE = 50;

/**
 * Storage key for schema state persistence
 * Used with Redux Persist for maintaining schema state across sessions
 */
export const SCHEMA_STATE_PERSIST_KEY = "folddb_schemas";

// ============================================================================
// REDUX ACTION TYPE CONSTANTS
// ============================================================================

/**
 * Base action types for schema slice
 */
export const SCHEMA_ACTION_TYPES = {
  // Async thunk action types
  FETCH_SCHEMAS: "schemas/fetchSchemas",
  APPROVE_SCHEMA: "schemas/approveSchema",
  BLOCK_SCHEMA: "schemas/blockSchema",
  UNLOAD_SCHEMA: "schemas/unloadSchema",
  LOAD_SCHEMA: "schemas/loadSchema",

  // Synchronous action types
  SET_ACTIVE_SCHEMA: "schemas/setActiveSchema",
  UPDATE_SCHEMA_STATUS: "schemas/updateSchemaStatus",
  SET_LOADING: "schemas/setLoading",
  SET_ERROR: "schemas/setError",
  CLEAR_ERROR: "schemas/clearError",
  CLEAR_OPERATION_ERROR: "schemas/clearOperationError",
  INVALIDATE_CACHE: "schemas/invalidateCache",
  RESET_SCHEMAS: "schemas/resetSchemas",
};

// ============================================================================
// SCHEMA STATE KEY CONSTANTS
// ============================================================================

/**
 * State keys for schema slice structure
 */
export const SCHEMA_STATE_KEYS = {
  SCHEMAS: "schemas",
  LOADING: "loading",
  ERRORS: "errors",
  LAST_FETCHED: "lastFetched",
  CACHE: "cache",
  ACTIVE_SCHEMA: "activeSchema",
  OPERATIONS: "operations",
};

/**
 * Nested state keys for loading states
 */
export const SCHEMA_LOADING_KEYS = {
  FETCH: "fetch",
  OPERATIONS: "operations",
};

/**
 * Nested state keys for error states
 */
export const SCHEMA_ERROR_KEYS = {
  FETCH: "fetch",
  OPERATIONS: "operations",
};

/**
 * Cache-related state keys
 */
export const SCHEMA_CACHE_KEYS = {
  TTL: "ttl",
  VERSION: "version",
  LAST_UPDATED: "lastUpdated",
};

// ============================================================================
// DEFAULT STATE VALUES
// ============================================================================

/**
 * Default loading state structure
 */
export const DEFAULT_LOADING_STATE = {
  [SCHEMA_LOADING_KEYS.FETCH]: false,
  [SCHEMA_LOADING_KEYS.OPERATIONS]: {},
};

/**
 * Default error state structure
 */
export const DEFAULT_ERROR_STATE = {
  [SCHEMA_ERROR_KEYS.FETCH]: null,
  [SCHEMA_ERROR_KEYS.OPERATIONS]: {},
};

/**
 * Default cache configuration
 */
export const DEFAULT_CACHE_STATE = {
  [SCHEMA_CACHE_KEYS.TTL]: SCHEMA_CACHE_TTL_MS,
  [SCHEMA_CACHE_KEYS.VERSION]: "1.0.0",
  [SCHEMA_CACHE_KEYS.LAST_UPDATED]: null,
};

/**
 * Complete default schema state
 */
export const DEFAULT_SCHEMA_STATE = {
  schemas: {},
  loading: {
    fetch: false,
    operations: {},
  },
  errors: {
    fetch: null,
    operations: {},
  },
  lastFetched: null,
  cache: {
    ttl: SCHEMA_CACHE_TTL_MS,
    version: "1.0.0",
    lastUpdated: null,
  },
  activeSchema: null,
};

// ============================================================================
// ERROR MESSAGE CONSTANTS
// ============================================================================

/**
 * Standard error messages for schema operations
 */
export const SCHEMA_ERROR_MESSAGES = {
  // Network and API errors
  FETCH_FAILED: "Failed to fetch schemas from server",
  NETWORK_ERROR: "Network error occurred while fetching schemas",
  API_TIMEOUT: "Request timed out - please try again",
  UNAUTHORIZED: "Not authorized to perform this operation",

  // Schema operation errors
  APPROVE_FAILED: "Failed to approve schema",
  BLOCK_FAILED: "Failed to block schema",
  UNLOAD_FAILED: "Failed to unload schema",
  LOAD_FAILED: "Failed to load schema",

  // Validation errors
  SCHEMA_NOT_FOUND: "Schema not found",
  INVALID_SCHEMA_STATE: "Invalid schema state for operation",
  SCHEMA_ALREADY_APPROVED: "Schema is already approved",
  SCHEMA_ALREADY_BLOCKED: "Schema is already blocked",

  // Cache and persistence errors
  CACHE_INVALIDATION_FAILED: "Failed to invalidate schema cache",
  PERSISTENCE_ERROR: "Failed to persist schema state",

  // General errors
  UNKNOWN_ERROR: "An unknown error occurred",
  OPERATION_CANCELLED: "Operation was cancelled by user",
};

// ============================================================================
// SCHEMA STATES AND VALIDATION CONSTANTS
// ============================================================================

/**
 * Valid schema states - extends base SCHEMA_STATES with Redux-specific UI states
 */
import { SCHEMA_STATES } from "./schemas";
export { SCHEMA_STATES };

/**
 * Schema operations that require specific states
 */
export const SCHEMA_OPERATION_REQUIREMENTS = {
  [SCHEMA_ACTION_TYPES.APPROVE_SCHEMA]: [
    SCHEMA_STATES.AVAILABLE,
    SCHEMA_STATES.BLOCKED,
  ],
  [SCHEMA_ACTION_TYPES.BLOCK_SCHEMA]: [
    SCHEMA_STATES.AVAILABLE,
    SCHEMA_STATES.APPROVED,
  ],
  [SCHEMA_ACTION_TYPES.UNLOAD_SCHEMA]: [
    SCHEMA_STATES.APPROVED,
    SCHEMA_STATES.BLOCKED,
  ],
};

/**
 * Schema states that allow read operations (SCHEMA-002 compliance)
 */
export const READABLE_SCHEMA_STATES = [SCHEMA_STATES.APPROVED];

/**
 * Schema states that allow write operations (SCHEMA-002 compliance)
 */
export const WRITABLE_SCHEMA_STATES = [SCHEMA_STATES.APPROVED];

// ============================================================================
// PERFORMANCE AND OPTIMIZATION CONSTANTS
// ============================================================================

/**
 * Debounce delay for schema search operations (ms)
 */
export const SCHEMA_SEARCH_DEBOUNCE_MS = 300;

/**
 * Maximum number of concurrent schema operations
 */
export const MAX_CONCURRENT_OPERATIONS = 3;

/**
 * Retry delay for failed operations (ms)
 */
export const OPERATION_RETRY_DELAY_MS = 1000;

/**
 * Maximum payload size for schema data (bytes)
 */
export const MAX_SCHEMA_PAYLOAD_SIZE = 1024 * 1024; // 1MB

// ============================================================================
// SELECTOR MEMOIZATION CONSTANTS
// ============================================================================

/**
 * Cache size for memoized selectors
 */
export const SELECTOR_CACHE_SIZE = 100;

/**
 * Equality check options for selector optimization
 */
export const SELECTOR_EQUALITY_OPTIONS = {
  maxSize: SELECTOR_CACHE_SIZE,
  equalityCheck: "shallow",
};

// ============================================================================
// MIDDLEWARE CONFIGURATION CONSTANTS
// ============================================================================

/**
 * Redux middleware configuration for schema operations
 */
export const SCHEMA_MIDDLEWARE_CONFIG = {
  // Logging configuration
  ENABLE_ACTION_LOGGING: process.env.NODE_ENV === "development",
  ENABLE_STATE_LOGGING: process.env.NODE_ENV === "development",

  // Performance monitoring
  ENABLE_PERFORMANCE_TRACKING: true,
  PERFORMANCE_THRESHOLD_MS: 100,

  // Error handling
  ENABLE_ERROR_REPORTING: true,
  ERROR_RETRY_ATTEMPTS: SCHEMA_FETCH_RETRY_ATTEMPTS,
};

/**
 * Development-only constants
 */
export const DEV_CONSTANTS = {
  // Mock data for testing
  MOCK_SCHEMA_COUNT: 10,
  MOCK_OPERATION_DELAY_MS: 500,

  // Debug flags
  ENABLE_REDUX_DEVTOOLS: process.env.NODE_ENV === "development",
  ENABLE_TIME_TRAVEL_DEBUGGING: process.env.NODE_ENV === "development",
};
