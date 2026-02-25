/**
 * Application-wide Configuration Constants
 */

// ============================================================================
// APPLICATION CONFIGURATION
// ============================================================================

/**
 * Default application settings and behavior
 */
export const APP_CONFIG = {
  // Default tab when application loads
  DEFAULT_TAB: 'smart-folder',
  
  // Authentication configuration
  AUTHENTICATION: {
    SESSION_TIMEOUT_MS: 3600000, // 1 hour
    KEY_REFRESH_INTERVAL_MS: 300000, // 5 minutes
    REQUIRE_AUTH_FOR_TABS: true
  },
  
  // Cache configuration
  CACHE: {
    DEFAULT_TTL_MS: 300000, // 5 minutes
    MAX_ENTRIES: 1000,
    CLEANUP_INTERVAL_MS: 60000, // 1 minute
    SCHEMA_TTL_MS: 600000 // 10 minutes
  },
  
  // Logging configuration
  LOGGING: {
    MAX_LOG_ENTRIES: 500,
    LOG_LEVELS: ['error', 'warn', 'info', 'debug'],
    ENABLE_CONSOLE_LOGGING: process.env.NODE_ENV === 'development'
  },
  
  // Performance settings
  PERFORMANCE: {
    DEBOUNCE_DELAY_MS: 300,
    SEARCH_DEBOUNCE_MS: 500,
    ANIMATION_DURATION_MS: 200,
    SLOW_ANIMATION_DURATION_MS: 500,
    FAST_ANIMATION_DURATION_MS: 100
  },
  
  // Feature flags
  FEATURES: {
    ENABLE_RANGE_SCHEMAS: true,
    ENABLE_SCHEMA_VALIDATION: true,
    ENABLE_BATCH_OPERATIONS: true,
    ENABLE_REAL_TIME_UPDATES: false
  }
};

// ============================================================================
// ENVIRONMENT CONFIGURATION
// ============================================================================

/**
 * Environment-specific settings
 */
export const ENVIRONMENT_CONFIG = {
  DEVELOPMENT: {
    API_BASE_URL: '/api',
    ENABLE_DEBUG_LOGGING: true,
    ENABLE_MOCK_DATA: false,
    CACHE_TTL_MS: 60000 // 1 minute for development
  },
  
  PRODUCTION: {
    API_BASE_URL: '/api',
    ENABLE_DEBUG_LOGGING: false,
    ENABLE_MOCK_DATA: false,
    CACHE_TTL_MS: 300000 // 5 minutes for production
  }
};

/**
 * Get current environment configuration
 */
export const getCurrentEnvironmentConfig = () => {
  const env = process.env.NODE_ENV || 'development';
  return ENVIRONMENT_CONFIG[env.toUpperCase()] || ENVIRONMENT_CONFIG.DEVELOPMENT;
};

// ============================================================================
// BROWSER AND DEVICE CONFIGURATION
// ============================================================================

/**
 * Browser compatibility and device settings
 */
export const BROWSER_CONFIG = {
  // Local storage keys (single source of truth)
  STORAGE_KEYS: {
    USER_ID: 'fold_user_id',
    USER_HASH: 'fold_user_hash',
    ONBOARDING_COMPLETED: 'folddb_onboarding_completed',
  },
  
  // Viewport and responsive settings
  VIEWPORT: {
    MIN_WIDTH: 320,
    TABLET_BREAKPOINT: 768,
    DESKTOP_BREAKPOINT: 1024,
    LARGE_SCREEN_BREAKPOINT: 1440
  },
  
  // Browser feature detection
  FEATURES: {
    SUPPORTS_LOCAL_STORAGE: typeof Storage !== 'undefined',
    SUPPORTS_WEB_WORKERS: typeof Worker !== 'undefined',
    SUPPORTS_WEBSOCKETS: typeof WebSocket !== 'undefined'
  }
};

// ============================================================================
// SECURITY CONFIGURATION
// ============================================================================

/**
 * Security-related configuration constants
 */
export const SECURITY_CONFIG = {
  // Cryptographic settings
  CRYPTO: {
    KEY_SIZE_BITS: 2048,
    SIGNATURE_ALGORITHM: 'SHA-256',
    ENCODING: 'base64'
  },
  
  // Request security
  REQUEST: {
    MAX_PAYLOAD_SIZE: 1048576, // 1MB
    TIMEOUT_MS: 30000,
    RETRY_ATTEMPTS: 3
  },
  
  // Content Security Policy
  CSP: {
    ALLOWED_ORIGINS: ['self'],
    ALLOWED_SCRIPTS: ['self', 'unsafe-inline'],
    ALLOWED_STYLES: ['self', 'unsafe-inline']
  }
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
  APP_CONFIG,
  ENVIRONMENT_CONFIG,
  BROWSER_CONFIG,
  SECURITY_CONFIG,
  getCurrentEnvironmentConfig
};