/**
 * Test Store Utilities
 * TASK-010: Test Suite Fixes and Validation
 *
 * Provides pre-configured Redux store instances for testing
 *
 * This file now imports from the consolidated testUtilities.jsx to eliminate duplication
 */

// Import consolidated utilities from testUtilities.jsx
import {
  createTestStore,
  renderWithRedux,
  renderHookWithRedux,
  createTestSchemaState,
  createMockAuthState
} from './testUtilities.jsx';

// Re-export utilities
export {
  createTestStore,
  renderWithRedux,
  renderHookWithRedux,
  createTestSchemaState,
  createMockAuthState
};

/**
 * Mock API responses for testing
 */
export const mockApiResponses = {
  schemas: {
    available: [
      { id: 'schema1', name: 'Test Schema 1', approved: false },
      { id: 'schema2', name: 'Test Schema 2', approved: false }
    ],
    approved: [
      { id: 'schema3', name: 'Approved Schema', approved: true }
    ]
  },
  fields: {
    schema1: [
      { name: 'field1', type: 'string' },
      { name: 'field2', type: 'number' }
    ]
  }
};

export default {
  createTestStore,
  renderWithRedux,
  renderHookWithRedux,
  createTestSchemaState,
  mockApiResponses
};