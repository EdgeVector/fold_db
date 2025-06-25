/**
 * Code Quality Configuration Constants
 * Required by TASK-011 Section 2.1.12
 */

// ESLint Configuration Constants
export const ESLINT_MAX_WARNINGS = 0;
export const TYPESCRIPT_STRICT_MODE = true;
export const ACCESSIBILITY_VIOLATION_THRESHOLD = 0;
export const JSDOC_COVERAGE_THRESHOLD_PERCENT = 90;
export const CODE_QUALITY_BATCH_SIZE = 15;

// Linting Batch Processing Configuration
export const LINTING_CONFIG = {
  maxWarnings: ESLINT_MAX_WARNINGS,
  strictMode: TYPESCRIPT_STRICT_MODE,
  accessibilityThreshold: ACCESSIBILITY_VIOLATION_THRESHOLD,
  docCoverageThreshold: JSDOC_COVERAGE_THRESHOLD_PERCENT,
  batchSize: CODE_QUALITY_BATCH_SIZE,
};

// File Pattern Constants for Linting
export const LINTING_PATTERNS = {
  testFiles: '**/*.{test,spec}.{js,jsx,ts,tsx}',
  componentFiles: '**/components/**/*.{js,jsx,ts,tsx}',
  hookFiles: '**/hooks/**/*.{js,jsx,ts,tsx}',
  utilFiles: '**/utils/**/*.{js,jsx,ts,tsx}',
  typeFiles: '**/types/**/*.{ts,tsx}',
  apiFiles: '**/api/**/*.{js,jsx,ts,tsx}',
};