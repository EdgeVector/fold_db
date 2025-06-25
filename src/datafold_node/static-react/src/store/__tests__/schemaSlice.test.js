/**
 * Redux Schema Slice Tests
 * TASK-003: State Management Consolidation with Redux
 */

import { configureStore } from '@reduxjs/toolkit';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import schemaReducer, {
  fetchSchemas,
  approveSchema,
  blockSchema,
  unloadSchema,
  loadSchema,
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

// Mock console to avoid noise in tests
global.console = {
  ...console,
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

describe('schemaSlice', () => {
  let store;

  beforeEach(() => {
    store = configureStore({
      reducer: {
        schemas: schemaReducer,
      },
    });
    vi.clearAllMocks();
    global.fetch = vi.fn();
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
      it('should handle successful fetch', async () => {
        // Mock the API responses
        global.fetch
          .mockResolvedValueOnce({
            ok: true,
            json: async () => ({ data: ['schema1', 'schema2'] })
          })
          .mockResolvedValueOnce({
            ok: true,
            json: async () => ({ data: { schema1: 'available', schema2: 'approved' } })
          })
          .mockResolvedValueOnce({
            ok: true,
            json: async () => ({ fields: {} })
          })
          .mockResolvedValueOnce({
            ok: true,
            json: async () => ({ fields: {} })
          });

        await store.dispatch(fetchSchemas());

        const state = store.getState().schemas;
        expect(state.loading.fetch).toBe(false);
        expect(state.errors.fetch).toBeNull();
        expect(Object.keys(state.schemas)).toHaveLength(2);
      });

      it('should handle fetch failure', async () => {
        global.fetch.mockRejectedValue(new Error('Network error'));

        await store.dispatch(fetchSchemas());

        const state = store.getState().schemas;
        expect(state.loading.fetch).toBe(false);
        expect(state.errors.fetch).toContain('Network error');
      });

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
        expect(fetch).not.toHaveBeenCalled();
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

      it('should handle approveSchema success', async () => {
        global.fetch.mockResolvedValue({
          ok: true,
          json: async () => ({ success: true, data: { schema: { name: 'test-schema', state: 'approved' } } })
        });

        await store.dispatch(approveSchema({ schemaName: 'test-schema' }));

        const state = store.getState().schemas;
        expect(state.schemas['test-schema'].state).toBe('approved');
        expect(state.loading.operations['test-schema']).toBe(false);
        expect(state.errors.operations['test-schema']).toBeUndefined();
      });

      it('should handle blockSchema success', async () => {
        global.fetch.mockResolvedValue({
          ok: true,
          json: async () => ({ success: true, data: { schema: { name: 'test-schema', state: 'blocked' } } })
        });

        await store.dispatch(blockSchema({ schemaName: 'test-schema' }));

        const state = store.getState().schemas;
        expect(state.schemas['test-schema'].state).toBe('blocked');
      });

      it('should handle operation failure', async () => {
        global.fetch.mockResolvedValue({
          ok: false,
          status: 500,
          statusText: 'Internal Server Error'
        });

        await store.dispatch(approveSchema({ schemaName: 'test-schema' }));

        const state = store.getState().schemas;
        expect(state.loading.operations['test-schema']).toBe(false);
        expect(state.errors.operations['test-schema']).toContain('500');
      });

      it('should handle operation on non-existent schema', async () => {
        await store.dispatch(approveSchema({ schemaName: 'non-existent' }));

        const state = store.getState().schemas;
        expect(state.errors.operations['non-existent']).toContain('Schema not found');
      });
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

  describe('error handling', () => {
    it('should handle network timeouts', async () => {
      const timeoutError = new Error('Operation timed out');
      global.fetch.mockRejectedValue(timeoutError);

      await store.dispatch(fetchSchemas());

      const state = store.getState().schemas;
      expect(state.errors.fetch).toContain('Operation timed out');
    });

    it('should handle malformed API responses', async () => {
      global.fetch.mockResolvedValue({
        ok: true,
        json: async () => {
          throw new Error('Invalid JSON response');
        }
      });

      await store.dispatch(fetchSchemas());

      const state = store.getState().schemas;
      expect(state.errors.fetch).toBeTruthy();
    });
  });

  describe('cache management', () => {
    it('should respect cache TTL', async () => {
      const oldTimestamp = Date.now() - 400000; // Older than 5 minutes
      
      store.dispatch(fetchSchemas.fulfilled({
        schemas: [{ name: 'cached-schema', state: 'available' }],
        timestamp: oldTimestamp
      }, '', undefined));

      // Mock fresh API response
      global.fetch
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ data: ['fresh-schema'] })
        })
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ data: { 'fresh-schema': 'available' } })
        })
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ fields: {} })
        });

      await store.dispatch(fetchSchemas());

      const state = store.getState().schemas;
      expect(state.lastFetched).toBeGreaterThan(oldTimestamp);
    });
  });
});