/**
 * Error Messages and Codes
 * TASK-005: Constants Extraction and Configuration Centralization
 * Section 2.1.12 - Use of Constants for Repeated or Special Values
 */

// ============================================================================
// ERROR CODES
// ============================================================================

/**
 * Standardized error codes for the application
 */
export const ERROR_CODES = {
  // General errors
  UNKNOWN_ERROR: 'UNKNOWN_ERROR',
  NETWORK_ERROR: 'NETWORK_ERROR',
  TIMEOUT_ERROR: 'TIMEOUT_ERROR',
  VALIDATION_ERROR: 'VALIDATION_ERROR',
  
  // Authentication errors
  AUTH_REQUIRED: 'AUTH_REQUIRED',
  AUTH_INVALID: 'AUTH_INVALID',
  AUTH_EXPIRED: 'AUTH_EXPIRED',
  AUTH_FORBIDDEN: 'AUTH_FORBIDDEN',
  
  // Schema errors (SCHEMA-002 compliance)
  SCHEMA_NOT_FOUND: 'SCHEMA_NOT_FOUND',
  SCHEMA_NOT_APPROVED: 'SCHEMA_NOT_APPROVED',
  SCHEMA_INVALID_STATE: 'SCHEMA_INVALID_STATE',
  SCHEMA_ALREADY_EXISTS: 'SCHEMA_ALREADY_EXISTS',
  SCHEMA_OPERATION_FAILED: 'SCHEMA_OPERATION_FAILED',
  
  // API errors
  API_ERROR: 'API_ERROR',
  API_NOT_FOUND: 'API_NOT_FOUND',
  API_BAD_REQUEST: 'API_BAD_REQUEST',
  API_SERVER_ERROR: 'API_SERVER_ERROR',
  API_RATE_LIMITED: 'API_RATE_LIMITED',
  
  // Form errors
  FORM_VALIDATION_FAILED: 'FORM_VALIDATION_FAILED',
  FORM_SUBMISSION_FAILED: 'FORM_SUBMISSION_FAILED',
  FORM_FIELD_REQUIRED: 'FORM_FIELD_REQUIRED',
  FORM_FIELD_INVALID: 'FORM_FIELD_INVALID',
  
  // Range schema errors
  RANGE_KEY_REQUIRED: 'RANGE_KEY_REQUIRED',
  RANGE_KEY_INVALID: 'RANGE_KEY_INVALID',
  RANGE_FILTER_INVALID: 'RANGE_FILTER_INVALID',
  
  // File upload errors
  FILE_TOO_LARGE: 'FILE_TOO_LARGE',
  FILE_INVALID_TYPE: 'FILE_INVALID_TYPE',
  FILE_UPLOAD_FAILED: 'FILE_UPLOAD_FAILED',
  
  // Database errors
  DB_CONNECTION_FAILED: 'DB_CONNECTION_FAILED',
  DB_QUERY_FAILED: 'DB_QUERY_FAILED',
  DB_TRANSACTION_FAILED: 'DB_TRANSACTION_FAILED'
};

// ============================================================================
// ERROR MESSAGES
// ============================================================================

/**
 * User-friendly error messages mapped to error codes
 */
export const ERROR_MESSAGES = {
  // General error messages
  [ERROR_CODES.UNKNOWN_ERROR]: 'An unexpected error occurred. Please try again.',
  [ERROR_CODES.NETWORK_ERROR]: 'Network connection failed. Please check your internet connection and try again.',
  [ERROR_CODES.TIMEOUT_ERROR]: 'The request timed out. Please try again.',
  [ERROR_CODES.VALIDATION_ERROR]: 'Please correct the validation errors and try again.',
  
  // Authentication error messages
  [ERROR_CODES.AUTH_REQUIRED]: 'Authentication is required to access this feature.',
  [ERROR_CODES.AUTH_INVALID]: 'Invalid authentication credentials. Please check your login information.',
  [ERROR_CODES.AUTH_EXPIRED]: 'Your session has expired. Please log in again.',
  [ERROR_CODES.AUTH_FORBIDDEN]: 'You do not have permission to perform this action.',
  
  // Schema error messages (SCHEMA-002 compliance)
  [ERROR_CODES.SCHEMA_NOT_FOUND]: 'The requested schema was not found.',
  [ERROR_CODES.SCHEMA_NOT_APPROVED]: 'Only approved schemas can be used for this operation.',
  [ERROR_CODES.SCHEMA_INVALID_STATE]: 'The schema is not in a valid state for this operation.',
  [ERROR_CODES.SCHEMA_ALREADY_EXISTS]: 'A schema with this name already exists.',
  [ERROR_CODES.SCHEMA_OPERATION_FAILED]: 'The schema operation failed. Please try again.',
  
  // API error messages
  [ERROR_CODES.API_ERROR]: 'A server error occurred. Please try again later.',
  [ERROR_CODES.API_NOT_FOUND]: 'The requested resource was not found.',
  [ERROR_CODES.API_BAD_REQUEST]: 'Invalid request. Please check your input and try again.',
  [ERROR_CODES.API_SERVER_ERROR]: 'Internal server error. Please try again later.',
  [ERROR_CODES.API_RATE_LIMITED]: 'Too many requests. Please wait a moment before trying again.',
  
  // Form error messages
  [ERROR_CODES.FORM_VALIDATION_FAILED]: 'Please correct the form errors below.',
  [ERROR_CODES.FORM_SUBMISSION_FAILED]: 'Failed to submit the form. Please try again.',
  [ERROR_CODES.FORM_FIELD_REQUIRED]: 'This field is required.',
  [ERROR_CODES.FORM_FIELD_INVALID]: 'Please enter a valid value for this field.',
  
  // Range schema error messages
  [ERROR_CODES.RANGE_KEY_REQUIRED]: 'Range key is required for range schema operations.',
  [ERROR_CODES.RANGE_KEY_INVALID]: 'The range key format is invalid.',
  [ERROR_CODES.RANGE_FILTER_INVALID]: 'The range filter configuration is invalid.',
  
  // File upload error messages
  [ERROR_CODES.FILE_TOO_LARGE]: 'The file is too large. Please choose a smaller file.',
  [ERROR_CODES.FILE_INVALID_TYPE]: 'Invalid file type. Please choose a supported file format.',
  [ERROR_CODES.FILE_UPLOAD_FAILED]: 'File upload failed. Please try again.',
  
  // Database error messages
  [ERROR_CODES.DB_CONNECTION_FAILED]: 'Database connection failed. Please try again later.',
  [ERROR_CODES.DB_QUERY_FAILED]: 'Database query failed. Please try again.',
  [ERROR_CODES.DB_TRANSACTION_FAILED]: 'Database transaction failed. Please try again.'
};

// ============================================================================
// ERROR CATEGORIES
// ============================================================================

/**
 * Error categories for grouping and handling
 */
export const ERROR_CATEGORIES = {
  NETWORK: 'network',
  AUTHENTICATION: 'authentication',
  VALIDATION: 'validation',
  SCHEMA: 'schema',
  API: 'api',
  FORM: 'form',
  FILE: 'file',
  DATABASE: 'database',
  SYSTEM: 'system'
};

/**
 * Map error codes to their categories
 */
export const ERROR_CODE_CATEGORIES = {
  [ERROR_CODES.NETWORK_ERROR]: ERROR_CATEGORIES.NETWORK,
  [ERROR_CODES.TIMEOUT_ERROR]: ERROR_CATEGORIES.NETWORK,
  
  [ERROR_CODES.AUTH_REQUIRED]: ERROR_CATEGORIES.AUTHENTICATION,
  [ERROR_CODES.AUTH_INVALID]: ERROR_CATEGORIES.AUTHENTICATION,
  [ERROR_CODES.AUTH_EXPIRED]: ERROR_CATEGORIES.AUTHENTICATION,
  [ERROR_CODES.AUTH_FORBIDDEN]: ERROR_CATEGORIES.AUTHENTICATION,
  
  [ERROR_CODES.VALIDATION_ERROR]: ERROR_CATEGORIES.VALIDATION,
  [ERROR_CODES.FORM_VALIDATION_FAILED]: ERROR_CATEGORIES.VALIDATION,
  [ERROR_CODES.FORM_FIELD_REQUIRED]: ERROR_CATEGORIES.VALIDATION,
  [ERROR_CODES.FORM_FIELD_INVALID]: ERROR_CATEGORIES.VALIDATION,
  
  [ERROR_CODES.SCHEMA_NOT_FOUND]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.SCHEMA_NOT_APPROVED]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.SCHEMA_INVALID_STATE]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.SCHEMA_ALREADY_EXISTS]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.SCHEMA_OPERATION_FAILED]: ERROR_CATEGORIES.SCHEMA,
  
  [ERROR_CODES.RANGE_KEY_REQUIRED]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.RANGE_KEY_INVALID]: ERROR_CATEGORIES.SCHEMA,
  [ERROR_CODES.RANGE_FILTER_INVALID]: ERROR_CATEGORIES.SCHEMA,
  
  [ERROR_CODES.API_ERROR]: ERROR_CATEGORIES.API,
  [ERROR_CODES.API_NOT_FOUND]: ERROR_CATEGORIES.API,
  [ERROR_CODES.API_BAD_REQUEST]: ERROR_CATEGORIES.API,
  [ERROR_CODES.API_SERVER_ERROR]: ERROR_CATEGORIES.API,
  [ERROR_CODES.API_RATE_LIMITED]: ERROR_CATEGORIES.API,
  
  [ERROR_CODES.FORM_SUBMISSION_FAILED]: ERROR_CATEGORIES.FORM,
  
  [ERROR_CODES.FILE_TOO_LARGE]: ERROR_CATEGORIES.FILE,
  [ERROR_CODES.FILE_INVALID_TYPE]: ERROR_CATEGORIES.FILE,
  [ERROR_CODES.FILE_UPLOAD_FAILED]: ERROR_CATEGORIES.FILE,
  
  [ERROR_CODES.DB_CONNECTION_FAILED]: ERROR_CATEGORIES.DATABASE,
  [ERROR_CODES.DB_QUERY_FAILED]: ERROR_CATEGORIES.DATABASE,
  [ERROR_CODES.DB_TRANSACTION_FAILED]: ERROR_CATEGORIES.DATABASE,
  
  [ERROR_CODES.UNKNOWN_ERROR]: ERROR_CATEGORIES.SYSTEM
};

// ============================================================================
// ERROR RECOVERY STRATEGIES
// ============================================================================

/**
 * Recovery strategies for different types of errors
 */
export const ERROR_RECOVERY_STRATEGIES = {
  [ERROR_CATEGORIES.NETWORK]: {
    retry: true,
    maxRetries: 3,
    retryDelay: 1000,
    showRetryButton: true
  },
  
  [ERROR_CATEGORIES.AUTHENTICATION]: {
    retry: false,
    redirectToLogin: true,
    clearLocalData: true
  },
  
  [ERROR_CATEGORIES.VALIDATION]: {
    retry: false,
    highlightFields: true,
    focusFirstError: true
  },
  
  [ERROR_CATEGORIES.SCHEMA]: {
    retry: true,
    maxRetries: 1,
    refreshSchemas: true
  },
  
  [ERROR_CATEGORIES.API]: {
    retry: true,
    maxRetries: 2,
    retryDelay: 2000
  },
  
  [ERROR_CATEGORIES.FORM]: {
    retry: false,
    preserveFormData: true,
    showErrorDetails: true
  },
  
  [ERROR_CATEGORIES.FILE]: {
    retry: false,
    clearFileInput: true,
    showFileRequirements: true
  },
  
  [ERROR_CATEGORIES.DATABASE]: {
    retry: true,
    maxRetries: 1,
    showContactSupport: true
  },
  
  [ERROR_CATEGORIES.SYSTEM]: {
    retry: true,
    maxRetries: 1,
    showContactSupport: true
  }
};

// ============================================================================
// ERROR UTILITIES
// ============================================================================

/**
 * Utility functions for error handling
 */
export const ERROR_UTILS = {
  /**
   * Get user-friendly message for an error code
   */
  getMessage: (code) => {
    return ERROR_MESSAGES[code] || ERROR_MESSAGES[ERROR_CODES.UNKNOWN_ERROR];
  },
  
  /**
   * Get category for an error code
   */
  getCategory: (code) => {
    return ERROR_CODE_CATEGORIES[code] || ERROR_CATEGORIES.SYSTEM;
  },
  
  /**
   * Get recovery strategy for an error code
   */
  getRecoveryStrategy: (code) => {
    const category = ERROR_UTILS.getCategory(code);
    return ERROR_RECOVERY_STRATEGIES[category] || ERROR_RECOVERY_STRATEGIES[ERROR_CATEGORIES.SYSTEM];
  },
  
  /**
   * Check if an error is retryable
   */
  isRetryable: (code) => {
    const strategy = ERROR_UTILS.getRecoveryStrategy(code);
    return strategy.retry === true;
  },
  
  /**
   * Create standardized error object
   */
  createError: (code, message = null, details = null) => {
    return {
      code,
      message: message || ERROR_UTILS.getMessage(code),
      category: ERROR_UTILS.getCategory(code),
      timestamp: new Date().toISOString(),
      details
    };
  }
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
  ERROR_CODES,
  ERROR_MESSAGES,
  ERROR_CATEGORIES,
  ERROR_CODE_CATEGORIES,
  ERROR_RECOVERY_STRATEGIES,
  ERROR_UTILS
};