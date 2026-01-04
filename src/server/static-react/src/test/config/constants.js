// Test Configuration Constants
// TASK-010: Required test constants for PBI-REACT-SIMPLIFY-001

export const TEST_TIMEOUT_DEFAULT_MS = 1000; // 1 second timeout - fast but allows for legitimate delays
export const COVERAGE_THRESHOLD_PERCENT = 85;
export const INTEGRATION_TEST_RETRY_COUNT = 3;
export const MOCK_API_DELAY_MS = 0; // No delays for fast tests
export const TEST_VALIDATION_BATCH_SIZE = 10;

// Additional test configuration
export const TEST_CONFIG = {
  timeout: {
    default: TEST_TIMEOUT_DEFAULT_MS,
    integration: TEST_TIMEOUT_DEFAULT_MS * 2,
    e2e: TEST_TIMEOUT_DEFAULT_MS * 3
  },
  coverage: {
    threshold: COVERAGE_THRESHOLD_PERCENT,
    statements: COVERAGE_THRESHOLD_PERCENT,
    branches: COVERAGE_THRESHOLD_PERCENT,
    functions: COVERAGE_THRESHOLD_PERCENT,
    lines: COVERAGE_THRESHOLD_PERCENT
  },
  retry: {
    integration: INTEGRATION_TEST_RETRY_COUNT,
    flaky: 2
  },
  api: {
    mockDelay: MOCK_API_DELAY_MS,
    timeout: 5000
  },
  validation: {
    batchSize: TEST_VALIDATION_BATCH_SIZE
  }
};