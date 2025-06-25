/**
 * Schema-related constants
 * Section 2.1.12 - Use of Constants for Repeated or Special Values
 */

// Schema fetching and caching constants
export const SCHEMA_FETCH_RETRY_COUNT = 3;
export const SCHEMA_CACHE_DURATION_MS = 300000; // 5 minutes
export const FORM_VALIDATION_DEBOUNCE_MS = 500;
export const RANGE_SCHEMA_FIELD_PREFIX = 'range_';

// Testing constants (TASK-006 requirements)
export const TEST_TIMEOUT_MS = 10000;
export const MOCK_DELAY_MS = 100;
export const COVERAGE_THRESHOLD_PERCENT = 80;
export const INTEGRATION_TEST_BATCH_SIZE = 5;
export const DOCUMENTATION_VERSION = '2.0.0';

// Schema state constants
export const SCHEMA_STATES = {
  AVAILABLE: 'available',
  APPROVED: 'approved',
  BLOCKED: 'blocked'
};

// API endpoints
export const SCHEMA_API_ENDPOINTS = {
  AVAILABLE: '/api/schemas/available',
  PERSISTED: '/api/schemas',
  SCHEMA_DETAIL: '/api/schema'
};

// Validation error messages
export const VALIDATION_MESSAGES = {
  RANGE_KEY_REQUIRED: 'Range key is required for range schema mutations',
  RANGE_KEY_EMPTY: 'Range key cannot be empty',
  SCHEMA_NOT_APPROVED: 'Only approved schemas can be used for this operation',
  FIELD_REQUIRED: 'This field is required',
  INVALID_TYPE: 'Invalid value type for this field'
};

// Range schema constants
export const RANGE_SCHEMA_CONFIG = {
  FIELD_TYPE: 'Range',
  MUTATION_WRAPPER_KEY: 'value'
};

// Form field types for validation
export const FIELD_TYPES = {
  STRING: 'string',
  NUMBER: 'number',
  BOOLEAN: 'boolean',
  RANGE: 'Range'
};