/**
 * useQueryState Hook Tests
 * Tests for UCR-1-2: Extract custom hooks for query state management with Redux integration
 * Part of UTC-1 Test Coverage Enhancement - Hook Testing
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { Provider } from 'react-redux';
import { useQueryState } from '../useQueryState.js';
import { createTestStore } from '../../test/utils/testUtilities.jsx';
import { selectFetchLoading, selectAllSchemas, selectApprovedSchemas } from '../../store/schemaSlice';

// Mock the Redux store hooks
vi.mock('../../store/hooks', () => ({
  useAppSelector: vi.fn(),
  useAppDispatch: vi.fn(() => vi.fn())
}));

describe('useQueryState Hook', () => {
  let mockStore;
  let mockUseAppSelector;

  const mockSchemas = [
    {
      name: 'UserSchema',
      state: 'approved',
      schema_type: { Range: { range_key: 'range_field' } },
      key: { range_field: 'range_field' },
      fields: ['range_field', 'id', 'name', 'age']
    },
    {
      name: 'ProductSchema',
      state: 'approved',
      schema_type: { Single: {} },
      fields: ['product_id', 'price', 'category']
    },
    {
      name: 'BlockedSchema',
      state: 'blocked',
      schema_type: { Single: {} },
      fields: ['field1']
    }
  ];

  beforeEach(async () => {
    mockStore = await createTestStore();
    
    // Import the mocked hook
    const { useAppSelector } = await import('../../store/hooks');
    mockUseAppSelector = useAppSelector;

    // Set up default mock implementation
    mockUseAppSelector.mockImplementation((selector) => {
      if (selector.toString().includes('auth')) {
        return { isAuthenticated: true };
      }
      if (selector === selectAllSchemas) {
        return mockSchemas;
      }
      if (selector === selectApprovedSchemas) {
        return mockSchemas.filter(s => s.state === 'approved');
      }
      if (selector === selectFetchLoading) {
        return false;
      }
      return undefined;
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  const renderUseQueryState = () => {
    return renderHook(() => useQueryState(), {
      wrapper: ({ children }) => (
        <Provider store={mockStore}>
          {children}
        </Provider>
      )
    });
  };

  describe('initial state', () => {
    it('should initialize with empty state', () => {
      const { result } = renderUseQueryState();

      expect(result.current.state.selectedSchema).toBe('');
      expect(result.current.state.queryFields).toEqual([]);
      expect(result.current.state.rangeFilters).toEqual({});
      expect(result.current.state.rangeSchemaFilter).toEqual({});
      expect(result.current.state.rangeKeyValue).toBe('');
    });

    it('should provide approved schemas when authenticated', () => {
      const { result } = renderUseQueryState();

      expect(result.current.approvedSchemas).toHaveLength(2);
      expect(result.current.approvedSchemas[0].name).toBe('UserSchema');
      expect(result.current.approvedSchemas[1].name).toBe('ProductSchema');
    });

    it('should return empty schemas when not authenticated', () => {
      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: false };
        }
        return [];
      });

      const { result } = renderUseQueryState();

      expect(result.current.approvedSchemas).toEqual([]);
    });

    it('should filter out non-approved schemas', () => {
      const { result } = renderUseQueryState();

      const schemaNames = result.current.approvedSchemas.map(s => s.name);
      expect(schemaNames).not.toContain('BlockedSchema');
    });
  });

  describe('schema selection', () => {
    it('should update selected schema', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.setSelectedSchema('UserSchema');
      });

      expect(result.current.state.selectedSchema).toBe('UserSchema');
    });

    it('should handle schema change with handleSchemaChange', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.state.selectedSchema).toBe('UserSchema');
    });

    it('should auto-select all fields when schema is selected', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.state.queryFields).toEqual(['range_field', 'id', 'name', 'age']);
    });

    it('should clear fields when schema is deselected', () => {
      const { result } = renderUseQueryState();

      // First select a schema
      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      // Then deselect
      act(() => {
        result.current.handleSchemaChange('');
      });

      expect(result.current.state.queryFields).toEqual([]);
    });

    it('should clear filters when schema changes', () => {
      const { result } = renderUseQueryState();

      // Set some filters
      act(() => {
        result.current.setRangeFilters({ field1: { start: 'a', end: 'z' } });
        result.current.setRangeKeyValue('test');
        result.current.setRangeSchemaFilter({ start: 'x', end: 'y' });
      });

      // Change schema
      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.state.rangeFilters).toEqual({});
      expect(result.current.state.rangeKeyValue).toBe('');
      expect(result.current.state.rangeSchemaFilter).toEqual({});
    });
  });

  describe('field management', () => {
    beforeEach(() => {
      // Setup with a selected schema
    });

    it('should toggle field selection on', () => {
      const { result } = renderUseQueryState();

      // Start with empty fields
      act(() => {
        result.current.setQueryFields([]);
      });

      act(() => {
        result.current.toggleField('id');
      });

      expect(result.current.state.queryFields).toEqual(['id']);
    });

    it('should toggle field selection off', () => {
      const { result } = renderUseQueryState();

      // Start with field selected
      act(() => {
        result.current.setQueryFields(['id', 'name']);
      });

      act(() => {
        result.current.toggleField('id');
      });

      expect(result.current.state.queryFields).toEqual(['name']);
    });

    it('should set query fields directly', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.setQueryFields(['id', 'name', 'age']);
      });

      expect(result.current.state.queryFields).toEqual(['id', 'name', 'age']);
    });
  });

  describe('range filter management', () => {
    it('should handle range filter changes', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleRangeFilterChange('range_field', 'start', 'value1');
      });

      expect(result.current.state.rangeFilters).toEqual({
        range_field: { start: 'value1' }
      });
    });

    it('should handle multiple range filter properties', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleRangeFilterChange('range_field', 'start', 'value1');
        result.current.handleRangeFilterChange('range_field', 'end', 'value2');
        result.current.handleRangeFilterChange('range_field', 'key', 'exact_key');
      });

      expect(result.current.state.rangeFilters).toEqual({
        range_field: {
          start: 'value1',
          end: 'value2',
          key: 'exact_key'
        }
      });
    });

    it('should handle range filters for multiple fields', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleRangeFilterChange('field1', 'start', 'a');
        result.current.handleRangeFilterChange('field2', 'key', 'exact');
      });

      expect(result.current.state.rangeFilters).toEqual({
        field1: { start: 'a' },
        field2: { key: 'exact' }
      });
    });

    it('should set range filters directly', () => {
      const { result } = renderUseQueryState();

      const newFilters = {
        field1: { start: 'a', end: 'z' },
        field2: { key: 'exact' }
      };

      act(() => {
        result.current.setRangeFilters(newFilters);
      });

      expect(result.current.state.rangeFilters).toEqual(newFilters);
    });

    it('should set range schema filter', () => {
      const { result } = renderUseQueryState();

      const filter = { start: 'user_001', end: 'user_999' };

      act(() => {
        result.current.setRangeSchemaFilter(filter);
      });

      expect(result.current.state.rangeSchemaFilter).toEqual(filter);
    });

    it('should set range key value', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.setRangeKeyValue('user:123');
      });

      expect(result.current.state.rangeKeyValue).toBe('user:123');
    });
  });

  describe('schema analysis', () => {
    it('should identify range schema correctly', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.isRangeSchema).toBe(true);
      expect(result.current.rangeKey).toBe('range_field');
    });

    it('should identify non-range schema correctly', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('ProductSchema');
      });

      expect(result.current.isRangeSchema).toBe(false);
      expect(result.current.rangeKey).toBeNull();
    });

    it('returns selected schema object regardless of authentication', () => {
      mockUseAppSelector.mockImplementation((selector) => {
        if (selector === selectAllSchemas) {
          return mockSchemas;
        }
        if (selector === selectFetchLoading) {
          return false;
        }
        return undefined;
      });

      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.selectedSchemaObj).toEqual(mockSchemas[0]);
    });

    it('should return selected schema object when authenticated', () => {
      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('UserSchema');
      });

      expect(result.current.selectedSchemaObj).toEqual(mockSchemas[0]);
    });
  });

  describe('state clearing', () => {
    it('should clear all state', () => {
      const { result } = renderUseQueryState();

      // Set some state first
      act(() => {
        result.current.setSelectedSchema('UserSchema');
        result.current.setQueryFields(['id', 'name']);
        result.current.setRangeFilters({ field1: { start: 'a' } });
        result.current.setRangeKeyValue('test');
        result.current.setRangeSchemaFilter({ start: 'x' });
      });

      // Clear all state
      act(() => {
        result.current.clearState();
      });

      expect(result.current.state.selectedSchema).toBe('');
      expect(result.current.state.queryFields).toEqual([]);
      expect(result.current.state.rangeFilters).toEqual({});
      expect(result.current.state.rangeKeyValue).toBe('');
      expect(result.current.state.rangeSchemaFilter).toEqual({});
    });
  });

  describe('loading states', () => {
    it('should reflect schemas loading state', () => {
      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: true };
        }
        if (selector === selectFetchLoading) {
          return true;
        }
        return mockSchemas;
      });

      const { result } = renderUseQueryState();

      expect(result.current.schemasLoading).toBe(true);
    });
  });

  describe('schema state normalization', () => {
    it('should handle uppercase schema states', () => {
      const schemasWithUppercaseState = [
        { name: 'Schema1', state: 'APPROVED', fields: {} },
        { name: 'Schema2', state: 'BLOCKED', fields: {} }
      ];
      // selectApprovedSchemas normalizes case, so APPROVED matches
      const approvedOnly = [schemasWithUppercaseState[0]];

      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: true };
        }
        if (selector === selectAllSchemas) {
          return schemasWithUppercaseState;
        }
        if (selector === selectApprovedSchemas) {
          return approvedOnly;
        }
        return false;
      });

      const { result } = renderUseQueryState();

      expect(result.current.approvedSchemas).toHaveLength(1);
      expect(result.current.approvedSchemas[0].name).toBe('Schema1');
    });

    it('should handle schema state objects with toString', () => {
      const schemasWithObjectState = [
        {
          name: 'Schema1',
          state: { toString: () => 'approved' },
          fields: {}
        }
      ];

      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: true };
        }
        if (selector === selectAllSchemas) {
          return schemasWithObjectState;
        }
        if (selector === selectApprovedSchemas) {
          return schemasWithObjectState;
        }
        return false;
      });

      const { result } = renderUseQueryState();

      expect(result.current.approvedSchemas).toHaveLength(1);
      expect(result.current.approvedSchemas[0].name).toBe('Schema1');
    });
  });

  describe('error handling', () => {
    it('should handle missing schema fields gracefully', () => {
      const schemasWithoutFields = [
        { name: 'Schema1', state: 'approved' }
      ];

      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: true };
        }
        if (selector === selectAllSchemas) {
          return schemasWithoutFields;
        }
        return false;
      });

      const { result } = renderUseQueryState();

      act(() => {
        result.current.handleSchemaChange('Schema1');
      });

      expect(result.current.state.queryFields).toEqual([]);
      expect(result.current.isRangeSchema).toBe(false);
      expect(result.current.rangeKey).toBeNull();
    });

    it('should handle null/undefined schemas gracefully', () => {
      mockUseAppSelector.mockImplementation((selector) => {
        if (selector.toString().includes('auth')) {
          return { isAuthenticated: true };
        }
        return [];
      });

      const { result } = renderUseQueryState();

      expect(result.current.approvedSchemas).toEqual([]);
      expect(result.current.selectedSchemaObj).toBeNull();
    });
  });
});