/**
 * Form Validation Rules and Messages
 * TASK-005: Constants Extraction and Configuration Centralization
 * Section 2.1.12 - Use of Constants for Repeated or Special Values
 */

// ============================================================================
// VALIDATION RULES
// ============================================================================

/**
 * Common validation rules and patterns
 */
export const VALIDATION_RULES = {
  // Text field validation
  TEXT: {
    MIN_LENGTH: 1,
    MAX_LENGTH: 255,
    DEFAULT_MAX_LENGTH: 1000
  },
  
  // Password validation
  PASSWORD: {
    MIN_LENGTH: 8,
    MAX_LENGTH: 128,
    REQUIRE_UPPERCASE: true,
    REQUIRE_LOWERCASE: true,
    REQUIRE_NUMBERS: true,
    REQUIRE_SPECIAL_CHARS: false
  },
  
  // Schema name validation
  SCHEMA_NAME: {
    MIN_LENGTH: 3,
    MAX_LENGTH: 64,
    PATTERN: /^[a-zA-Z][a-zA-Z0-9_]*$/,
    RESERVED_WORDS: ['system', 'admin', 'root', 'default']
  },
  
  // Field name validation
  FIELD_NAME: {
    MIN_LENGTH: 1,
    MAX_LENGTH: 64,
    PATTERN: /^[a-zA-Z][a-zA-Z0-9_]*$/,
    RESERVED_WORDS: ['id', 'type', 'schema', 'mutation']
  },
  
  // Range key validation
  RANGE_KEY: {
    MIN_LENGTH: 1,
    MAX_LENGTH: 256,
    ALLOW_EMPTY_FOR_DELETE: true
  },
  
  // File upload validation
  FILE_UPLOAD: {
    MAX_SIZE_BYTES: 1048576, // 1MB
    ALLOWED_TYPES: ['application/json', 'text/plain'],
    ALLOWED_EXTENSIONS: ['.json', '.txt']
  },
  
  // Numeric validation
  NUMERIC: {
    MIN_SAFE_INTEGER: Number.MIN_SAFE_INTEGER,
    MAX_SAFE_INTEGER: Number.MAX_SAFE_INTEGER,
    DECIMAL_PLACES: 10
  }
};

/**
 * Regular expression patterns for validation
 */
export const VALIDATION_PATTERNS = {
  EMAIL: /^[^\s@]+@[^\s@]+\.[^\s@]+$/,
  URL: /^https?:\/\/(www\.)?[-a-zA-Z0-9@:%._+~#=]{1,256}\.[a-zA-Z0-9()]{1,6}\b([-a-zA-Z0-9()@:%_+.~#?&//=]*)$/,
  PHONE: /^\+?[\d\s\-()]{10,}$/,
  UUID: /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
  HEX_COLOR: /^#([A-Fa-f0-9]{6}|[A-Fa-f0-9]{3})$/,
  SLUG: /^[a-z0-9]+(?:-[a-z0-9]+)*$/,
  ALPHANUMERIC: /^[a-zA-Z0-9]+$/,
  ALPHANUMERIC_WITH_UNDERSCORE: /^[a-zA-Z0-9_]+$/
};

// ============================================================================
// VALIDATION MESSAGES
// ============================================================================

/**
 * Error messages for validation failures
 */
export const VALIDATION_MESSAGES = {
  // General validation messages
  REQUIRED: 'This field is required',
  INVALID_FORMAT: 'Invalid format',
  TOO_SHORT: 'Value is too short',
  TOO_LONG: 'Value is too long',
  INVALID_TYPE: 'Invalid value type for this field',
  
  // Schema-specific messages (SCHEMA-002 compliance)
  RANGE_KEY_REQUIRED: 'Range key is required for range schema mutations',
  RANGE_KEY_EMPTY: 'Range key cannot be empty',
  SCHEMA_NOT_APPROVED: 'Only approved schemas can be used for this operation',
  INVALID_SCHEMA_STATE: 'Schema is not in a valid state for this operation',
  SCHEMA_ALREADY_EXISTS: 'A schema with this name already exists',
  
  // Field validation messages
  FIELD_REQUIRED: 'This field is required',
  FIELD_TOO_SHORT: (min) => `Field must be at least ${min} characters long`,
  FIELD_TOO_LONG: (max) => `Field must not exceed ${max} characters`,
  FIELD_INVALID_PATTERN: 'Field contains invalid characters',
  FIELD_RESERVED_NAME: 'This field name is reserved and cannot be used',
  
  // File upload messages
  FILE_TOO_LARGE: (maxSize) => `File size must not exceed ${Math.round(maxSize / 1024)} KB`,
  FILE_INVALID_TYPE: 'Invalid file type',
  FILE_UPLOAD_FAILED: 'File upload failed',
  
  // Authentication messages
  INVALID_CREDENTIALS: 'Invalid username or password',
  SESSION_EXPIRED: 'Your session has expired. Please log in again.',
  UNAUTHORIZED_ACCESS: 'You do not have permission to access this resource',
  
  // Network and API messages
  NETWORK_ERROR: 'Network error occurred. Please check your connection.',
  API_ERROR: 'Server error occurred. Please try again later.',
  TIMEOUT_ERROR: 'Request timed out. Please try again.',
  
  // Form-specific messages
  FORM_INVALID: 'Please correct the errors below',
  FORM_SAVE_FAILED: 'Failed to save form data',
  FORM_RESET_CONFIRM: 'Are you sure you want to reset the form? All changes will be lost.',
  
  // Range schema specific messages
  RANGE_FILTER_INVALID: 'Invalid range filter configuration',
  RANGE_START_AFTER_END: 'Start value must be less than end value',
  RANGE_KEY_INVALID_FORMAT: 'Range key format is invalid'
};

/**
 * Success messages for validation and operations
 */
export const SUCCESS_MESSAGES = {
  FORM_SAVED: 'Form saved successfully',
  SCHEMA_APPROVED: 'Schema approved successfully',
  SCHEMA_BLOCKED: 'Schema blocked successfully',
  SCHEMA_LOADED: 'Schema loaded successfully',
  MUTATION_EXECUTED: 'Mutation executed successfully',
  QUERY_EXECUTED: 'Query executed successfully',
  FILE_UPLOADED: 'File uploaded successfully',
  CHANGES_SAVED: 'Changes saved successfully'
};

// ============================================================================
// VALIDATION CONFIGURATION
// ============================================================================

/**
 * Configuration for form validation behavior
 */
export const VALIDATION_CONFIG = {
  // Debounce settings
  DEBOUNCE: {
    DEFAULT_DELAY_MS: 300,
    SEARCH_DELAY_MS: 500,
    VALIDATION_DELAY_MS: 300
  },
  
  // Error display settings
  ERROR_DISPLAY: {
    SHOW_ON_BLUR: true,
    SHOW_ON_CHANGE: true,
    SHOW_ON_SUBMIT: true,
    CLEAR_ON_FOCUS: false,
    AUTO_HIDE_AFTER_MS: 5000
  },
  
  // Validation triggers
  TRIGGERS: {
    ON_CHANGE: 'onChange',
    ON_BLUR: 'onBlur',
    ON_SUBMIT: 'onSubmit',
    ON_FOCUS: 'onFocus'
  },
  
  // Field states
  FIELD_STATES: {
    IDLE: 'idle',
    VALIDATING: 'validating',
    VALID: 'valid',
    INVALID: 'invalid',
    PRISTINE: 'pristine',
    DIRTY: 'dirty'
  }
};

/**
 * Validation functions - REMOVED
 * Backend is authoritative for all validation
 * Frontend only prevents obviously pointless API calls
 */

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
  VALIDATION_RULES,
  VALIDATION_PATTERNS,
  VALIDATION_MESSAGES,
  SUCCESS_MESSAGES,
  VALIDATION_CONFIG
};