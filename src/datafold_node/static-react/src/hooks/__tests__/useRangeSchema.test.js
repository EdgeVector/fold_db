import { renderHook } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useRangeSchema } from '../useRangeSchema.js';
import { RANGE_SCHEMA_CONFIG, VALIDATION_MESSAGES } from '../../constants/schemas.js';

describe('useRangeSchema Hook', () => {
  let mockConsole;

  beforeEach(() => {
    vi.clearAllMocks();
    mockConsole = {
      warn: vi.fn(),
      log: vi.fn(),
      error: vi.fn()
    };
    global.console = { ...console, ...mockConsole };
  });

  const createMockRangeSchema = () => ({
    name: 'TestRangeSchema',
    schema_type: {
      Range: {
        range_key: 'timestamp'
      }
    },
    fields: {
      timestamp: { field_type: 'Range' },
      value: { field_type: 'Range' },
      metadata: { field_type: 'Range' }
    }
  });

  const createMockRegularSchema = () => ({
    name: 'TestRegularSchema',
    schema_type: { Standard: {} },
    fields: {
      id: { field_type: 'String' },
      count: { field_type: 'Number' },
      active: { field_type: 'Boolean' }
    }
  });

  const createMockOldFormatRangeSchema = () => ({
    name: 'OldFormatRangeSchema',
    range_key: 'date_key',
    fields: {
      date_key: { field_type: 'Range' },
      data: { field_type: 'Range' }
    }
  });

  it('should detect range schemas correctly', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();
    const regularSchema = createMockRegularSchema();

    expect(result.current.isRange(rangeSchema)).toBe(true);
    expect(result.current.isRange(regularSchema)).toBe(false);
  });

  it('should detect old format range schemas', () => {
    const { result } = renderHook(() => useRangeSchema());
    const oldFormatSchema = createMockOldFormatRangeSchema();

    expect(result.current.isRange(oldFormatSchema)).toBe(true);
  });

  it('should handle invalid schema inputs gracefully', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.isRange(null)).toBe(false);
    expect(result.current.isRange(undefined)).toBe(false);
    expect(result.current.isRange({})).toBe(false);
    expect(result.current.isRange({ fields: {} })).toBe(false);
    expect(result.current.isRange({ schema_type: {} })).toBe(false);
  });

  it('should validate mixed field types correctly', () => {
    const { result } = renderHook(() => useRangeSchema());
    
    const mixedSchema = {
      name: 'MixedSchema',
      schema_type: { Range: { range_key: 'key' } },
      fields: {
        key: { field_type: 'Range' },
        data: { field_type: 'Range' },
        metadata: { field_type: 'String' } // Not a Range field
      }
    };

    expect(result.current.isRange(mixedSchema)).toBe(false);
    expect(mockConsole.warn).toHaveBeenCalledWith(
      expect.stringContaining('Field metadata has field_type "String", expected "Range"')
    );
  });

  it('should provide range key extraction functionality', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();
    const oldFormatSchema = createMockOldFormatRangeSchema();

    expect(result.current.rangeProps.getRangeKey(rangeSchema)).toBe('timestamp');
    expect(result.current.rangeProps.getRangeKey(oldFormatSchema)).toBe('date_key');
    expect(result.current.rangeProps.getRangeKey(createMockRegularSchema())).toBe(null);
  });

  it('should extract non-range-key fields correctly', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const nonRangeKeyFields = result.current.rangeProps.getNonRangeKeyFields(rangeSchema);
    
    expect(nonRangeKeyFields).toHaveProperty('value');
    expect(nonRangeKeyFields).toHaveProperty('metadata');
    expect(nonRangeKeyFields).not.toHaveProperty('timestamp'); // Should be excluded
  });

  it('should validate range keys correctly', () => {
    const { result } = renderHook(() => useRangeSchema());

    // Valid range keys
    expect(result.current.rangeProps.validateRangeKey('valid_key', true)).toBe(null);
    expect(result.current.rangeProps.validateRangeKey('valid_key', false)).toBe(null);

    // Empty/whitespace range keys
    expect(result.current.rangeProps.validateRangeKey('', true)).toBe(VALIDATION_MESSAGES.RANGE_KEY_REQUIRED);
    expect(result.current.rangeProps.validateRangeKey('   ', true)).toBe(VALIDATION_MESSAGES.RANGE_KEY_EMPTY);
    expect(result.current.rangeProps.validateRangeKey(null, true)).toBe(VALIDATION_MESSAGES.RANGE_KEY_REQUIRED);
    expect(result.current.rangeProps.validateRangeKey(undefined, true)).toBe(VALIDATION_MESSAGES.RANGE_KEY_REQUIRED);

    // Optional range keys
    expect(result.current.rangeProps.validateRangeKey('', false)).toBe(null);
    expect(result.current.rangeProps.validateRangeKey(null, false)).toBe(null);
  });

  it('should format range mutations correctly for Create/Update', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const fieldData = {
      value: 42,
      metadata: 'test string'
    };

    const mutation = result.current.rangeProps.formatRangeMutation(
      rangeSchema,
      'Create',
      'test_key_123',
      fieldData
    );

    expect(mutation.type).toBe('mutation');
    expect(mutation.schema).toBe('TestRangeSchema');
    expect(mutation.mutation_type).toBe('create');
    expect(mutation.data.timestamp).toBe('test_key_123'); // Range key as primitive
    expect(mutation.data.value).toEqual({ [RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY]: 42 }); // Wrapped in object
    expect(mutation.data.metadata).toEqual({ [RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY]: 'test string' });
  });

  it('should format range mutations correctly for Delete', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const mutation = result.current.rangeProps.formatRangeMutation(
      rangeSchema,
      'Delete',
      'test_key_123',
      {}
    );

    expect(mutation.type).toBe('mutation');
    expect(mutation.schema).toBe('TestRangeSchema');
    expect(mutation.mutation_type).toBe('delete');
    expect(mutation.data.timestamp).toBe('test_key_123');
    expect(Object.keys(mutation.data)).toHaveLength(1); // Only range key
  });

  it('should handle different field value types in mutations', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const fieldData = {
      string_field: 'text',
      number_field: 123,
      boolean_field: true,
      object_field: { custom: 'data' },
      null_field: null
    };

    const mutation = result.current.rangeProps.formatRangeMutation(
      rangeSchema,
      'Create',
      'test_key',
      fieldData
    );

    expect(mutation.data.string_field).toEqual({ value: 'text' });
    expect(mutation.data.number_field).toEqual({ value: 123 });
    expect(mutation.data.boolean_field).toEqual({ value: true });
    expect(mutation.data.object_field).toEqual({ custom: 'data' }); // Objects used as-is
    expect(mutation.data.null_field).toEqual({ value: null });
  });

  it('should format range queries correctly', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const fields = ['value', 'metadata'];
    const query = result.current.rangeProps.formatRangeQuery(rangeSchema, fields, 'filter_key');

    expect(query.type).toBe('query');
    expect(query.schema).toBe('TestRangeSchema');
    expect(query.fields).toEqual(fields);
    expect(query.range_filter).toEqual({ Key: 'filter_key' });
  });

  it('should handle empty range filter in queries', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const query = result.current.rangeProps.formatRangeQuery(rangeSchema, ['value'], '');

    expect(query).not.toHaveProperty('range_filter');
  });

  it('should provide range schema information', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const info = result.current.rangeProps.getRangeSchemaInfo(rangeSchema);

    expect(info.isRangeSchema).toBe(true);
    expect(info.rangeKey).toBe('timestamp');
    expect(info.rangeFields).toHaveLength(3);
    expect(info.totalFields).toBe(3);
    expect(info.nonRangeKeyFields).toHaveProperty('value');
    expect(info.nonRangeKeyFields).not.toHaveProperty('timestamp');
  });

  it('should return null for non-range schema info', () => {
    const { result } = renderHook(() => useRangeSchema());
    const regularSchema = createMockRegularSchema();

    const info = result.current.rangeProps.getRangeSchemaInfo(regularSchema);

    expect(info).toBe(null);
  });

  it('should extract range and non-range fields separately', () => {
    const { result } = renderHook(() => useRangeSchema());
    const mixedSchema = {
      name: 'MixedSchema',
      fields: {
        range_field1: { field_type: 'Range' },
        range_field2: { field_type: 'Range' },
        string_field: { field_type: 'String' },
        number_field: { field_type: 'Number' }
      }
    };

    const rangeFields = result.current.rangeProps.getRangeFields(mixedSchema);
    const nonRangeFields = result.current.rangeProps.getNonRangeFields(mixedSchema);

    expect(rangeFields).toEqual(['range_field1', 'range_field2']);
    expect(nonRangeFields).toHaveProperty('string_field');
    expect(nonRangeFields).toHaveProperty('number_field');
    expect(nonRangeFields).not.toHaveProperty('range_field1');
    expect(nonRangeFields).not.toHaveProperty('range_field2');
  });

  it('should handle schemas without fields gracefully', () => {
    const { result } = renderHook(() => useRangeSchema());
    const emptySchema = { name: 'EmptySchema' };

    expect(result.current.rangeProps.getRangeFields(emptySchema)).toEqual([]);
    expect(result.current.rangeProps.getNonRangeFields(emptySchema)).toEqual({});
    expect(result.current.rangeProps.getRangeSchemaInfo(emptySchema)).toBe(null);
  });

  it('should provide placeholder min, max, step functions', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.min()).toBe(null);
    expect(result.current.max()).toBe(null);
    expect(result.current.step()).toBe(null);
  });

  it('should include debounce configuration in rangeProps', () => {
    const { result } = renderHook(() => useRangeSchema());

    expect(result.current.rangeProps.debounceMs).toBe(500); // FORM_VALIDATION_DEBOUNCE_MS
  });

  it('should handle whitespace in range key values', () => {
    const { result } = renderHook(() => useRangeSchema());
    const rangeSchema = createMockRangeSchema();

    const mutation = result.current.rangeProps.formatRangeMutation(
      rangeSchema,
      'Create',
      '  spaced_key  ',
      {}
    );

    expect(mutation.data.timestamp).toBe('spaced_key'); // Trimmed
  });
});