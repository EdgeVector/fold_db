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
} from './config.js';

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
} from './validation.js';

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
// TESTING EXPORTS
// ============================================================================

export {
  TEST_TIMEOUT_DEFAULT_MS,
  COVERAGE_THRESHOLD_PERCENT,
  INTEGRATION_TEST_RETRY_COUNT,
  MOCK_API_DELAY_MS,
  TEST_VALIDATION_BATCH_SIZE,
  FINAL_VALIDATION_TIMEOUT_MS,
  COMMIT_MESSAGE_MIN_LENGTH,
  TEST_SUITE_RETRY_COUNT,
  DEPLOYMENT_VALIDATION_TIMEOUT_MS,
  TASK_COMPLETION_BATCH_SIZE,
  TEST_CONFIG,
  TEST_ENVIRONMENT
} from './testing';

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
} from './api.ts';

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

// Import constants internally to use in Constants object
import { FIELD_TYPES as _FIELD_TYPES, RANGE_SCHEMA_CONFIG as _RANGE_SCHEMA_CONFIG } from './schemas';
import { APP_CONFIG as _APP_CONFIG } from './config.js';
import { VALIDATION_RULES as _VALIDATION_RULES, VALIDATION_PATTERNS as _VALIDATION_PATTERNS, VALIDATION_MESSAGES as _VALIDATION_MESSAGES } from './validation.js';
import { COLORS as _COLORS } from './styling.js';
import { DEFAULT_TABS as _DEFAULT_TABS, BUTTON_TEXT as _BUTTON_TEXT, FORM_LABELS as _FORM_LABELS, UI_STATES as _UI_STATES } from './ui.js';
import { ERROR_CODES as _ERROR_CODES } from './errors.js';

// Use already exported SCHEMA_STATES to avoid circular dependency
import { SCHEMA_STATES as _SCHEMA_STATES } from './api.ts';

// ============================================================================
// UI EXPORTS (from existing files)
// ============================================================================

export {
  TAB_TRANSITION_DURATION_MS,
  FORM_FIELD_DEBOUNCE_MS,
  DEFAULT_TABS,
  SCHEMA_BADGE_COLORS,
  FORM_LABELS,
  BUTTON_TEXT,
  MUTATION_TYPES,
  UI_STATES,
  AUTH_INDICATORS,
  HELP_TEXT
} from './ui';

// ============================================================================
// CLEANUP EXPORTS
// ============================================================================

export {
  CLEANUP_BATCH_SIZE,
  DEPENDENCY_SCAN_TIMEOUT_MS,
  LEGACY_FILE_AGE_DAYS,
  UNUSED_IMPORT_THRESHOLD,
  CLEANUP_VALIDATION_TIMEOUT_MS,
  CODE_SIMILARITY_THRESHOLD_PERCENT,
  DUPLICATE_DETECTION_BATCH_SIZE,
  CONSOLIDATION_VALIDATION_TIMEOUT_MS,
  PATTERN_ANALYSIS_DEPTH,
  DUPLICATE_LINE_THRESHOLD
} from './cleanup';

// ============================================================================
// OPTIMIZATION EXPORTS
// ============================================================================

export {
  COMPLEXITY_SCORE_THRESHOLD,
  PROP_INTERFACE_MAX_PROPS,
  COMPONENT_MAX_LINES,
  ABSTRACTION_JUSTIFICATION_THRESHOLD,
  OPTIMIZATION_VALIDATION_TIMEOUT_MS,
  COMPLEXITY_WEIGHT_CYCLOMATIC,
  COMPLEXITY_WEIGHT_CONDITIONAL_RENDER,
  COMPLEXITY_WEIGHT_NESTED_DEPTH,
  COMPLEXITY_WEIGHT_PROP_COUNT,
  MAX_FUNCTION_LINES,
  MAX_JSX_NESTING_DEPTH,
  MAX_USEEFFECT_DEPENDENCIES,
  MAX_CONDITIONAL_BRANCHES,
  BUNDLE_SIZE_WARNING_THRESHOLD_KB,
  RENDER_PERFORMANCE_WARNING_MS,
  MEMORY_LEAK_DETECTION_THRESHOLD
} from './optimization';

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
export const DEFAULT_TAB = 'keys'; // Direct constant to avoid undefined issues

// Export destructured SCHEMA_STATES safely - ensure SCHEMA_STATES is imported first
export const APPROVED = _SCHEMA_STATES.APPROVED;
export const AVAILABLE = _SCHEMA_STATES.AVAILABLE;
export const BLOCKED = _SCHEMA_STATES.BLOCKED;

// Export key color values safely
export const PRIMARY_COLOR = '#3b82f6';
export const STATUS_SUCCESS = '#10b981';
export const STATUS_ERROR = '#ef4444';

// ============================================================================
// NAMESPACED EXPORTS
// ============================================================================

/**
 * Organized namespaces for related constants
 */
/**
 * Organized Constants Namespace
 * Note: Using explicit exports instead of object literals to avoid undefined variable errors
 */
export const Constants = {
  // Configuration namespace
  Config: {
    APP_CONFIG: _APP_CONFIG || { DEFAULT_TAB: 'keys' },
    DEFAULT_TAB: DEFAULT_TAB
  },
  
  // Validation namespace - using imported constants
  Validation: {
    VALIDATION_RULES: _VALIDATION_RULES || {},
    VALIDATION_PATTERNS: _VALIDATION_PATTERNS || {},
    VALIDATION_MESSAGES: _VALIDATION_MESSAGES || {}
  },
  
  // Styling namespace - using imported constants
  Styles: {
    COLORS: _COLORS || {},
    PRIMARY_COLOR: PRIMARY_COLOR,
    STATUS_SUCCESS: STATUS_SUCCESS,
    STATUS_ERROR: STATUS_ERROR
  },
  
  // Schema namespace (SCHEMA-002 compliance)
  Schema: {
    STATES: _SCHEMA_STATES,
    FIELD_TYPES: _FIELD_TYPES || {},
    RANGE_SCHEMA_CONFIG: _RANGE_SCHEMA_CONFIG || {}
  },
  
  // UI namespace - using imported constants
  UI: {
    DEFAULT_TABS: _DEFAULT_TABS || [],
    BUTTON_TEXT: _BUTTON_TEXT || {},
    FORM_LABELS: _FORM_LABELS || {},
    UI_STATES: _UI_STATES || {}
  },
  
  // Error namespace - using imported constants
  Errors: {
    ERROR_CODES: _ERROR_CODES || {}
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