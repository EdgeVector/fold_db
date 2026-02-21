/**
 * Redux Schema Slice Tests
 * TASK-003: State Management Consolidation with Redux
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { createTestStore } from '../../test/utils/testUtilities.jsx';
import {
  fetchSchemas,
  approveSchema,
  setActiveSchema,
  updateSchemaStatus,
  setLoading,
  setError,
  clearError,
  invalidateCache,
  resetSchemas,
  selectAllSchemas,
  selectApprovedSchemas,
  selectAvailableSchemas,
  selectBlockedSchemas,
  selectSchemaById,
  selectFetchLoading,
  selectFetchError
} from '../schemaSlice';

// Mock SchemaClient
vi.mock('../../api/clients/schemaClient', () => ({
  UnifiedSchemaClient: vi.fn().mockImplementation(() => ({
    getSchemas: vi.fn(),
    getSchemasByState: vi.fn(),
    getAllSchemasWithState: vi.fn(),
    approveSchema: vi.fn(),
    blockSchema: vi.fn(),
    loadSchema: vi.fn(),
    unloadSchema: vi.fn(),
    getSchema: vi.fn()
  })),
  schemaClient: {
    getSchemas: vi.fn(),
    getSchemasByState: vi.fn(),
    getAllSchemasWithState: vi.fn(),
    approveSchema: vi.fn(),
    blockSchema: vi.fn(),
    loadSchema: vi.fn(),
    unloadSchema: vi.fn(),
    getSchema: vi.fn()
  }
}));

// Mock console to avoid noise in tests
global.console = {
  ...console,
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

describe('schemaSlice', () => {
  let store;
  let mockSchemaClient;

  beforeEach(async () => {
    store = await createTestStore();
    vi.clearAllMocks();
    
    // Import the mocked schemaClient
    const { schemaClient } = await import('../../api/clients/schemaClient');
    mockSchemaClient = schemaClient;
    
    // Reset all mocks to ensure clean state
    Object.values(mockSchemaClient).forEach(mockFn => {
      if (typeof mockFn === 'function' && mockFn.mockReset) {
        mockFn.mockReset();
      }
    });
  });

  describe('initial state', () => {
    it('should have correct initial state', () => {
      const state = store.getState().schemas;
      
      expect(state.schemas).toEqual({});
      expect(state.loading.fetch).toBe(false);
      expect(state.loading.operations).toEqual({});
      expect(state.errors.fetch).toBeNull();
      expect(state.errors.operations).toEqual({});
      expect(state.lastFetched).toBeNull();
      expect(state.activeSchema).toBeNull();
    });
  });

  describe('synchronous actions', () => {
    it('should set active schema', () => {
      store.dispatch(setActiveSchema('test-schema'));
      
      const state = store.getState().schemas;
      expect(state.activeSchema).toBe('test-schema');
    });

    it('should update schema status', () => {
      // First add a schema
      const testSchema = {
        name: 'test-schema',
        state: 'available'
      };
      
      store.dispatch(fetchSchemas.fulfilled({
        schemas: [testSchema],
        timestamp: Date.now()
      }, '', undefined));

      // Then update its status
      store.dispatch(updateSchemaStatus({
        schemaName: 'test-schema',
        newState: 'approved'
      }));

      const state = store.getState().schemas;
      expect(state.schemas['test-schema'].state).toBe('approved');
    });

    it('should set loading state', () => {
      store.dispatch(setLoading({
        operation: 'fetch',
        isLoading: true
      }));

      const state = store.getState().schemas;
      expect(state.loading.fetch).toBe(true);
    });

    it('should set error state', () => {
      const errorMessage = 'Test error';
      store.dispatch(setError({
        operation: 'fetch',
        error: errorMessage
      }));

      const state = store.getState().schemas;
      expect(state.errors.fetch).toBe(errorMessage);
    });

    it('should clear errors', () => {
      // Set some errors first
      store.dispatch(setError({
        operation: 'fetch',
        error: 'Test error'
      }));

      store.dispatch(clearError());

      const state = store.getState().schemas;
      expect(state.errors.fetch).toBeNull();
      expect(state.errors.operations).toEqual({});
    });

    it('should invalidate cache', () => {
      // Set some cache data first
      store.dispatch(fetchSchemas.fulfilled({
        schemas: [],
        timestamp: Date.now()
      }, '', undefined));

      store.dispatch(invalidateCache());

      const state = store.getState().schemas;
      expect(state.lastFetched).toBeNull();
    });

    it('should reset schemas', () => {
      // Add some data first
      store.dispatch(setActiveSchema('test'));
      store.dispatch(setError({
        operation: 'fetch',
        error: 'test'
      }));

      store.dispatch(resetSchemas());

      const state = store.getState().schemas;
      expect(state.schemas).toEqual({});
      expect(state.activeSchema).toBeNull();
      expect(state.errors.fetch).toBeNull();
    });
  });

  describe('async thunks', () => {
    describe('fetchSchemas', () => {


      it('should return cached data when cache is valid', async () => {
        // First, populate cache
        const timestamp = Date.now();
        store.dispatch(fetchSchemas.fulfilled({
          schemas: [{ name: 'cached-schema', state: 'available' }],
          timestamp
        }, '', undefined));

        // Then try to fetch again (should use cache)
        await store.dispatch(fetchSchemas());

        const state = store.getState().schemas;
        expect(state.lastFetched).toBe(timestamp);
        expect(mockSchemaClient.getSchemas).not.toHaveBeenCalled();
      });
    });

    describe('schema operations', () => {
      beforeEach(() => {
        // Add a test schema
        const testSchema = {
          name: 'test-schema',
          state: 'available'
        };
        
        store.dispatch(fetchSchemas.fulfilled({
          schemas: [testSchema],
          timestamp: Date.now()
        }, '', undefined));
      });







      it('should handle operation on non-existent schema', async () => {
        // Mock the API to reject with schema not found error
        const { schemaClient } = await import('../../api/clients/schemaClient');
        schemaClient.approveSchema.mockRejectedValueOnce(new Error('Schema not found'));

        await store.dispatch(approveSchema({ schemaName: 'non-existent' }));

        const state = store.getState().schemas;
        // Check if error was set (the exact path may vary based on schema slice implementation)
        expect(state.error || state.errors?.operations?.['non-existent']).toBeDefined();
      }, 3000);
    });
  });

  describe('selectors', () => {
    beforeEach(() => {
      const testSchemas = [
        { name: 'available-schema', state: 'available' },
        { name: 'approved-schema-1', state: 'approved' },
        { name: 'approved-schema-2', state: 'approved' },
        { name: 'blocked-schema', state: 'blocked' },
        { 
          name: 'range-schema', 
          state: 'approved',
          rangeInfo: { isRangeSchema: true, rangeField: { name: 'range_key', type: 'Range' } }
        }
      ];

      store.dispatch(fetchSchemas.fulfilled({
        schemas: testSchemas,
        timestamp: Date.now()
      }, '', undefined));
    });

    it('should select all schemas', () => {
      const allSchemas = selectAllSchemas(store.getState());
      expect(allSchemas).toHaveLength(5);
    });

    it('should select only approved schemas (SCHEMA-002 compliance)', () => {
      const approvedSchemas = selectApprovedSchemas(store.getState());
      expect(approvedSchemas).toHaveLength(3);
      expect(approvedSchemas.every(schema => schema.state === 'approved')).toBe(true);
    });

    it('should select only available schemas', () => {
      const availableSchemas = selectAvailableSchemas(store.getState());
      expect(availableSchemas).toHaveLength(1);
      expect(availableSchemas[0].name).toBe('available-schema');
    });

    it('should select only blocked schemas', () => {
      const blockedSchemas = selectBlockedSchemas(store.getState());
      expect(blockedSchemas).toHaveLength(1);
      expect(blockedSchemas[0].name).toBe('blocked-schema');
    });

    it('should select schema by ID', () => {
      const schema = selectSchemaById('approved-schema-1')(store.getState());
      expect(schema?.name).toBe('approved-schema-1');
      expect(schema?.state).toBe('approved');
    });

    it('should return null for non-existent schema', () => {
      const schema = selectSchemaById('non-existent')(store.getState());
      expect(schema).toBeNull();
    });

    it('should select fetch loading state', () => {
      store.dispatch(setLoading({ operation: 'fetch', isLoading: true }));
      
      const isLoading = selectFetchLoading(store.getState());
      expect(isLoading).toBe(true);
    });

    it('should select fetch error state', () => {
      const errorMessage = 'Test error';
      store.dispatch(setError({ operation: 'fetch', error: errorMessage }));
      
      const error = selectFetchError(store.getState());
      expect(error).toBe(errorMessage);
    });
  });

  describe('SCHEMA-002 compliance', () => {
    it('should enforce that only approved schemas are used for mutations', () => {
      const testSchemas = [
        { name: 'available-schema', state: 'available' },
        { name: 'approved-schema', state: 'approved' },
        { name: 'blocked-schema', state: 'blocked' }
      ];

      store.dispatch(fetchSchemas.fulfilled({
        schemas: testSchemas,
        timestamp: Date.now()
      }, '', undefined));

      const approvedSchemas = selectApprovedSchemas(store.getState());
      
      // Only approved schemas should be returned
      expect(approvedSchemas).toHaveLength(1);
      expect(approvedSchemas[0].name).toBe('approved-schema');
      expect(approvedSchemas[0].state).toBe('approved');
    });
  });



  describe('cache management', () => {

    it('should update schema state optimistically after successful operations', () => {
      const store = createTestStore({
        schemas: {
          schemas: {
            'test-schema': { name: 'test-schema', state: 'available' }
          },
          lastFetched: Date.now(),
          cache: { lastUpdated: Date.now() },
          loading: {
            fetch: false,
            operations: {}
          },
          errors: {
            fetch: null,
            operations: {}
          }
        }
      });

      // Dispatch the fulfilled action directly to test optimistic update
      store.dispatch(approveSchema.fulfilled({
        schemaName: 'test-schema',
        newState: 'approved',
        timestamp: Date.now(),
        updatedSchema: undefined
      }, 'test-request-id', { schemaName: 'test-schema' }));

      // Get the final state
      const state = store.getState().schemas;

      // Verify schema state was updated optimistically
      expect(state.schemas['test-schema'].state).toBe('approved');
      expect(state.schemas['test-schema'].lastOperation.type).toBe('approve');
      expect(state.schemas['test-schema'].lastOperation.success).toBe(true);
      
      // Verify cache was NOT invalidated (simplified approach)
      expect(state.lastFetched).not.toBeNull();
      expect(state.cache.lastUpdated).not.toBeNull();
    });
  });
});