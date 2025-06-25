/**
 * Testing Configuration Constants
 * TASK-010: Test Suite Fixes and Validation
 * Section 2.1.12 - Required Constants for Testing
 */

// ============================================================================
// TEST EXECUTION CONSTANTS
// ============================================================================

export const TEST_TIMEOUT_DEFAULT_MS = 15000;
export const COVERAGE_THRESHOLD_PERCENT = 85;
export const INTEGRATION_TEST_RETRY_COUNT = 3;
export const MOCK_API_DELAY_MS = 100;
export const TEST_VALIDATION_BATCH_SIZE = 10;

// ============================================================================
// TASK-012: FINAL VALIDATION CONSTANTS (Section 2.1.12)
// ============================================================================

export const FINAL_VALIDATION_TIMEOUT_MS = 120000;
export const COMMIT_MESSAGE_MIN_LENGTH = 50;
export const TEST_SUITE_RETRY_COUNT = 2;
export const DEPLOYMENT_VALIDATION_TIMEOUT_MS = 180000;
export const TASK_COMPLETION_BATCH_SIZE = 6;

// ============================================================================
// TEST CONFIGURATION
// ============================================================================

export const TEST_CONFIG = {
  TIMEOUT: {
    DEFAULT: TEST_TIMEOUT_DEFAULT_MS,
    INTEGRATION: 30000,
    UNIT: 5000,
    HOOK: 10000
  },
  
  COVERAGE: {
    THRESHOLD: COVERAGE_THRESHOLD_PERCENT,
    STATEMENTS: 80,
    BRANCHES: 75,
    FUNCTIONS: 80,
    LINES: 80
  },
  
  MOCK: {
    API_DELAY: MOCK_API_DELAY_MS,
    NETWORK_DELAY: 50,
    USER_INTERACTION_DELAY: 100
  },
  
  RETRY: {
    INTEGRATION_TESTS: INTEGRATION_TEST_RETRY_COUNT,
    FLAKY_TESTS: 2,
    NETWORK_TESTS: 3
  },
  
  BATCH: {
    VALIDATION_SIZE: TEST_VALIDATION_BATCH_SIZE,
    MOCK_DATA_SIZE: 5,
    CONCURRENT_TESTS: 4
  }
};

// ============================================================================
// TEST ENVIRONMENT CONSTANTS
// ============================================================================

export const TEST_ENVIRONMENT = {
  JSDOM_URL: 'http://localhost:3000',
  API_BASE_URL: 'http://localhost:9001',
  MOCK_API_URL: 'http://localhost:8080/api'
};

// ============================================================================
// DEFAULT EXPORT
// ============================================================================

export default {
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
};