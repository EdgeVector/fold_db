/**
 * Centralized Constants Index
 * TASK-005: Constants Extraction and Configuration Centralization
 * 
 * This file provides a centralized export of all constants for easy importing
 * and maintains an organized namespace structure for different constant categories.
 * 
 * Usage Examples:
 * 
 * // Import specific constant categories
 * import { APP_CONFIG, VALIDATION_RULES } from '@/constants';
 * 
 * // Import all constants as namespaced object
 * import Constants from '@/constants';
 * const tabId = Constants.APP_CONFIG.DEFAULT_TAB;
 * 
 * // Import specific constants directly
 * import { DEFAULT_TAB, SCHEMA_STATES } from '@/constants';
 */

// ============================================================================
// CONFIGURATION EXPORTS
// ============================================================================

export {
  APP_CONFIG,
  ENVIRONMENT_CONFIG,
  BROWSER_CONFIG,
  SECURITY_CONFIG,
  getCurrentEnvironmentConfig
} from './config';

// ============================================================================
// VALIDATION EXPORTS
// ============================================================================

export {
  VALIDATION_RULES,
  VALIDATION_PATTERNS,
  VALIDATION_MESSAGES,
  SUCCESS_MESSAGES,
  VALIDATION_CONFIG,
  VALIDATION_FUNCTIONS
} from './validation';

// ============================================================================
// STYLING EXPORTS
// ============================================================================

export {
  COLORS,
  LAYOUT,
  TYPOGRAPHY,
  COMPONENT_STYLES,
  ANIMATIONS,
  BREAKPOINTS,
  Z_INDEX
} from './styling';

// ============================================================================
// ERROR EXPORTS
// ============================================================================

export {
  ERROR_CODES,
  ERROR_MESSAGES,
  ERROR_CATEGORIES,
  ERROR_CODE_CATEGORIES,
  ERROR_RECOVERY_STRATEGIES,
  ERROR_UTILS
} from './errors';

// ============================================================================
// API EXPORTS (from existing files)
// ============================================================================

export {
  API_REQUEST_TIMEOUT_MS,
  API_RETRY_ATTEMPTS,
  API_RETRY_DELAY_MS,
  API_BATCH_REQUEST_LIMIT,
  HTTP_STATUS_CODES,
  CONTENT_TYPES,
  REQUEST_HEADERS,
  ERROR_MESSAGES as API_ERROR_MESSAGES,
  CACHE_CONFIG,
  RETRY_CONFIG,
  API_CONFIG,
  SCHEMA_STATES as API_SCHEMA_STATES,
  SCHEMA_OPERATIONS
} from './api';

// ============================================================================
// SCHEMA EXPORTS (from existing files)
// ============================================================================

export {
  SCHEMA_FETCH_RETRY_COUNT,
  SCHEMA_CACHE_DURATION_MS,
  FORM_VALIDATION_DEBOUNCE_MS,
  RANGE_SCHEMA_FIELD_PREFIX,
  SCHEMA_STATES,
  SCHEMA_API_ENDPOINTS,
  VALIDATION_MESSAGES as SCHEMA_VALIDATION_MESSAGES,
  RANGE_SCHEMA_CONFIG,
  FIELD_TYPES
} from './schemas';

// ============================================================================
// UI EXPORTS (from existing files)
// ============================================================================

export {
  TAB_TRANSITION_DURATION_MS,
  FORM_FIELD_DEBOUNCE_MS,
  DEFAULT_TABS,
  SCHEMA_BADGE_COLORS,
  COMPONENT_Z_INDEX,
  FORM_LABELS,
  BUTTON_TEXT,
  MUTATION_TYPES,
  FIELD_TYPE_CONFIG,
  PERMISSION_COLORS,
  UI_STATES,
  RANGE_SCHEMA_CONFIG as UI_RANGE_SCHEMA_CONFIG,
  COMPONENT_STYLES as UI_COMPONENT_STYLES,
  AUTH_INDICATORS,
  HELP_TEXT,
  BREAKPOINTS as UI_BREAKPOINTS
} from './ui';

// ============================================================================
// REDUX EXPORTS (from existing files)
// ============================================================================

export {
  SCHEMA_CACHE_TTL_MS,
  SCHEMA_FETCH_RETRY_ATTEMPTS,
  SCHEMA_OPERATION_TIMEOUT_MS,
  REDUX_BATCH_SIZE,
  SCHEMA_STATE_PERSIST_KEY,
  SCHEMA_ACTION_TYPES,
  SCHEMA_STATE_KEYS,
  SCHEMA_LOADING_KEYS,
  SCHEMA_ERROR_KEYS,
  SCHEMA_CACHE_KEYS,
  DEFAULT_LOADING_STATE,
  DEFAULT_ERROR_STATE,
  DEFAULT_CACHE_STATE,
  DEFAULT_SCHEMA_STATE,
  SCHEMA_ERROR_MESSAGES,
  SCHEMA_STATES as REDUX_SCHEMA_STATES,
  SCHEMA_OPERATION_REQUIREMENTS,
  READABLE_SCHEMA_STATES,
  WRITABLE_SCHEMA_STATES,
  SCHEMA_SEARCH_DEBOUNCE_MS,
  MAX_CONCURRENT_OPERATIONS,
  OPERATION_RETRY_DELAY_MS,
  MAX_SCHEMA_PAYLOAD_SIZE,
  SELECTOR_CACHE_SIZE,
  SELECTOR_EQUALITY_OPTIONS,
  SCHEMA_MIDDLEWARE_CONFIG,
  DEV_CONSTANTS
} from './redux';

// ============================================================================
// CONVENIENCE EXPORTS - FREQUENTLY USED CONSTANTS
// ============================================================================

// Most commonly used constants for easy access
export const {
  DEFAULT_TAB
} = APP_CONFIG;

export const {
  APPROVED,
  AVAILABLE,
  BLOCKED
} = SCHEMA_STATES;

export const {
  PRIMARY,
  STATUS,
  SCHEMA_STATES: SCHEMA_STATE_COLORS
} = COLORS;

export const {
  BASE,
  PRIMARY: PRIMARY_BUTTON,
  SECONDARY,
  DANGER
} = COMPONENT_STYLES.BUTTON;

// ============================================================================
// NAMESPACED EXPORTS
// ============================================================================

/**
 * Organized namespaces for related constants
 */
export const Constants = {
  // Configuration namespace
  Config: {
    APP_CONFIG,
    ENVIRONMENT_CONFIG,
    BROWSER_CONFIG,
    SECURITY_CONFIG
  },
  
  // Validation namespace
  Validation: {
    VALIDATION_RULES,
    VALIDATION_PATTERNS,
    VALIDATION_MESSAGES,
    VALIDATION_FUNCTIONS
  },
  
  // Styling namespace
  Styles: {
    COLORS,
    LAYOUT,
    TYPOGRAPHY,
    COMPONENT_STYLES,
    ANIMATIONS,
    BREAKPOINTS,
    Z_INDEX
  },
  
  // Error handling namespace
  Errors: {
    ERROR_CODES,
    ERROR_MESSAGES,
    ERROR_CATEGORIES,
    ERROR_UTILS
  },
  
  // Schema namespace (SCHEMA-002 compliance)
  Schema: {
    STATES: SCHEMA_STATES,
    VALIDATION_MESSAGES: VALIDATION_MESSAGES,
    FIELD_TYPES,
    RANGE_SCHEMA_CONFIG
  },
  
  // UI namespace
  UI: {
    DEFAULT_TABS,
    BUTTON_TEXT,
    FORM_LABELS,
    UI_STATES,
    MUTATION_TYPES
  },
  
  // API namespace
  API: {
    HTTP_STATUS_CODES,
    CONTENT_TYPES,
    REQUEST_HEADERS,
    CACHE_CONFIG,
    RETRY_CONFIG
  }
};

// ============================================================================
// CONSTANTS METADATA
// ============================================================================

/**
 * Metadata about the constants system for debugging and documentation
 */
export const CONSTANTS_METADATA = {
  VERSION: '1.0.0',
  LAST_UPDATED: '2025-06-24',
  TOTAL_FILES: 6,
  CATEGORIES: [
    'Configuration',
    'Validation',
    'Styling',
    'Errors',
    'API',
    'Schema',
    'UI',
    'Redux'
  ],
  MIGRATION_STATUS: {
    CONFIG_EXTRACTED: true,
    VALIDATION_EXTRACTED: true,
    STYLING_EXTRACTED: true,
    ERRORS_EXTRACTED: true,
    HARDCODED_VALUES_REMAINING: false
  }
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

/**
 * Default export provides the complete constants namespace
 */
export default Constants;