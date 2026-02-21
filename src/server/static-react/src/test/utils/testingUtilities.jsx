/**
 * @fileoverview Testing Utilities for React Application
 *
 * This file now imports from the consolidated testUtilities.jsx to eliminate duplication.
 * All testing utilities are now centralized in testUtilities.jsx
 *
 * TASK-006: Testing Enhancement - Consolidated testing utilities
 *
 * @module testingUtilities
 * @since 2.0.0
 */

// Import all utilities from the consolidated testUtilities.jsx
import {
  createTestStore,
  renderWithRedux as renderWithProviders,
  createMockSchema,
  createMockRangeSchema,
  createMockSchemaList,
  createMockAuthState,
  waitForCondition,
  mockDelay,
  createMockError,
  validateCoverage,
  createTestBatch,
  mockLocalStorage,
  mockSessionStorage,
  toBeValidSchema,
  setupTestEnvironment,
  cleanupTestEnvironment
} from './testUtilities.jsx';

// Re-export utilities
export {
  createTestStore,
  renderWithProviders,
  createMockSchema,
  createMockRangeSchema,
  createMockSchemaList,
  createMockAuthState,
  waitForCondition,
  mockDelay,
  createMockError,
  validateCoverage,
  createTestBatch,
  mockLocalStorage,
  mockSessionStorage,
  toBeValidSchema,
  setupTestEnvironment,
  cleanupTestEnvironment
};

// Note: Mock creation functions are defined below in the consolidated section

// Note: All utility functions are defined in the consolidated section below

// Export all utilities as default
export default {
  createTestStore,
  renderWithProviders,
  createMockSchema,
  createMockRangeSchema,
  createMockSchemaList,
  createMockAuthState,
  waitForCondition,
  mockDelay,
  createMockError,
  validateCoverage,
  createTestBatch,
  mockLocalStorage,
  mockSessionStorage,
  toBeValidSchema,
  setupTestEnvironment,
  cleanupTestEnvironment
};