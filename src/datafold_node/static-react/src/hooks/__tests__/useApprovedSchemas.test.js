import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useApprovedSchemas } from '../useApprovedSchemas.js';
import { SCHEMA_STATES, SCHEMA_CACHE_DURATION_MS, SCHEMA_FETCH_RETRY_COUNT } from '../../constants/schemas.js';

// Mock console to avoid noise in tests
global.console = {
  ...console,
  log: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
};

describe('useApprovedSchemas Hook', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.fetch = vi.fn();
  });

  const mockAvailableSchemasResponse = {
    ok: true,
    json: async () => ({ data: ['TestSchema1', 'TestSchema2', 'RangeSchema'] })
  };

  const mockPersistedSchemasResponse = {
    ok: true,
    json: async () => ({ 
      data: { 
        'TestSchema1': 'approved',
        'TestSchema2': 'available',
        'RangeSchema': 'approved'
      } 
    })
  };

  const mockSchemaDetailResponses = [
    {
      ok: true,
      json: async () => ({
        name: 'TestSchema1',
        fields: { field1: { field_type: 'String' } },
        schema_type: { Standard: {} }
      })
    },
    {
      ok: false, // Simulate one schema not loaded in memory
      status: 404
    },
    {
      ok: true,
      json: async () => ({
        name: 'RangeSchema',
        fields: { 
          range_key: { field_type: 'Range' },
          data_field: { field_type: 'Range' }
        },
        schema_type: { Range: { range_key: 'range_key' } }
      })
    }
  ];

  it('should fetch and return approved schemas on mount', async () => {
    // Setup fetch mock responses
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result } = renderHook(() => useApprovedSchemas());

    // Initially loading
    expect(result.current.isLoading).toBe(true);
    expect(result.current.approvedSchemas).toEqual([]);
    expect(result.current.error).toBe(null);

    // Wait for fetch to complete
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Should only return approved schemas (SCHEMA-002 compliance)
    expect(result.current.approvedSchemas).toHaveLength(2);
    expect(result.current.approvedSchemas[0].name).toBe('TestSchema1');
    expect(result.current.approvedSchemas[0].state).toBe('approved');
    expect(result.current.approvedSchemas[1].name).toBe('RangeSchema');
    expect(result.current.approvedSchemas[1].state).toBe('approved');

    // Should have all schemas available
    expect(result.current.allSchemas).toHaveLength(3);
    
    expect(result.current.error).toBe(null);
  });

  it('should enforce SCHEMA-002 compliance by filtering only approved schemas', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Only approved schemas should be returned
    const approvedNames = result.current.approvedSchemas.map(s => s.name);
    expect(approvedNames).toEqual(['TestSchema1', 'RangeSchema']);
    expect(approvedNames).not.toContain('TestSchema2'); // This one is 'available'
  });

  it('should provide isSchemaApproved function that works correctly', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.isSchemaApproved('TestSchema1')).toBe(true);
    expect(result.current.isSchemaApproved('TestSchema2')).toBe(false);
    expect(result.current.isSchemaApproved('RangeSchema')).toBe(true);
    expect(result.current.isSchemaApproved('NonExistentSchema')).toBe(false);
  });

  it('should provide getSchemaByName function', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const schema1 = result.current.getSchemaByName('TestSchema1');
    expect(schema1).toBeTruthy();
    expect(schema1.name).toBe('TestSchema1');
    expect(schema1.state).toBe('approved');

    const nonExistent = result.current.getSchemaByName('NonExistent');
    expect(nonExistent).toBe(null);
  });

  it('should handle fetch errors with retry logic', async () => {
    const fetchError = new Error('Network error');
    
    // First three attempts fail, fourth succeeds
    fetch
      .mockRejectedValueOnce(fetchError)
      .mockRejectedValueOnce(fetchError)
      .mockRejectedValueOnce(fetchError)
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse);

    const { result } = renderHook(() => useApprovedSchemas());

    // Wait for retries to complete
    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    }, { timeout: 5000 });

    // Should have error after max retries
    expect(result.current.error).toContain(`Failed to fetch schemas after ${SCHEMA_FETCH_RETRY_COUNT} attempts`);
    expect(result.current.approvedSchemas).toEqual([]);
  });

  it('should use cache for subsequent calls within cache duration', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result, rerender } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const initialCallCount = fetch.mock.calls.length;

    // Rerender the hook (simulating component re-render)
    rerender();

    // Should not make additional fetch calls due to cache
    expect(fetch.mock.calls.length).toBe(initialCallCount);
    expect(result.current.isLoading).toBe(false);
    expect(result.current.approvedSchemas).toHaveLength(2);
  });

  it('should support manual refetch that bypasses cache', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    const initialCallCount = fetch.mock.calls.length;

    // Setup fresh responses for refetch
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    // Manual refetch should bypass cache
    await act(async () => {
      await result.current.refetch();
    });

    expect(fetch.mock.calls.length).toBeGreaterThan(initialCallCount);
  });

  it('should handle different state formats correctly', async () => {
    const mixedStateResponse = {
      ok: true,
      json: async () => ({ 
        data: { 
          'Schema1': 'APPROVED', // uppercase
          'Schema2': { state: 'approved' }, // object format
          'Schema3': 'Available' // mixed case
        } 
      })
    };

    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: ['Schema1', 'Schema2', 'Schema3'] })
      })
      .mockResolvedValueOnce(mixedStateResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[0]);

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    // Should normalize states correctly
    expect(result.current.isSchemaApproved('Schema1')).toBe(true); // APPROVED -> approved
    expect(result.current.isSchemaApproved('Schema2')).toBe(true); // object format
    expect(result.current.isSchemaApproved('Schema3')).toBe(false); // Available != approved
  });

  it('should handle API endpoint failures gracefully', async () => {
    fetch.mockResolvedValueOnce({
      ok: false,
      status: 500
    });

    const { result } = renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(result.current.isLoading).toBe(false);
    });

    expect(result.current.error).toContain('Failed to fetch available schemas: 500');
    expect(result.current.approvedSchemas).toEqual([]);
  });

  it('should log appropriate console messages during operation', async () => {
    fetch
      .mockResolvedValueOnce(mockAvailableSchemasResponse)
      .mockResolvedValueOnce(mockPersistedSchemasResponse)
      .mockResolvedValueOnce(mockSchemaDetailResponses[0])
      .mockResolvedValueOnce(mockSchemaDetailResponses[1])
      .mockResolvedValueOnce(mockSchemaDetailResponses[2]);

    renderHook(() => useApprovedSchemas());

    await waitFor(() => {
      expect(console.log).toHaveBeenCalledWith('📁 Available schemas:', expect.any(Array));
      expect(console.log).toHaveBeenCalledWith('🗄️ Persisted schemas:', expect.any(Object));
      expect(console.log).toHaveBeenCalledWith('📋 Merged schemas for UI:', expect.any(Array));
      expect(console.log).toHaveBeenCalledWith('✅ Final schemas for UI:', expect.any(Array));
    });
  });
});