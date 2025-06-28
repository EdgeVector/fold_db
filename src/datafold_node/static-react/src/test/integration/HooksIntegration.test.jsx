import React from 'react';
import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useApprovedSchemas, useRangeSchema, useFormValidation } from '../../hooks/index.js';
import { SCHEMA_STATES } from '../../constants/schemas.js';
import { renderHookWithRedux } from '../utils/testStore.jsx';

describe('Hooks Integration Tests', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    global.fetch = vi.fn();
    global.console = {
      ...console,
      log: vi.fn(),
      warn: vi.fn(),
      error: vi.fn()
    };
  });

  const mockApprovedSchemasResponse = () => {
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: ['RegularSchema', 'RangeSchema'] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: { 
            'RegularSchema': 'approved',
            'RangeSchema': 'approved'
          } 
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'RegularSchema',
          fields: { 
            name: { field_type: 'String' },
            count: { field_type: 'Number' }
          },
          schema_type: { Standard: {} }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'RangeSchema',
          fields: { 
            timestamp: { field_type: 'Range' },
            value: { field_type: 'Range' }
          },
          schema_type: { Range: { range_key: 'timestamp' } }
        })
      });
  };

  it('should integrate useApprovedSchemas with useRangeSchema for complete schema operations', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: rangeResult } = renderHook(() => useRangeSchema());

    // Wait for schemas to load
    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const approvedSchemas = schemasResult.current.approvedSchemas;
    expect(approvedSchemas).toHaveLength(2);

    // Test regular schema operations
    const regularSchema = approvedSchemas.find(s => s.name === 'RegularSchema');
    expect(regularSchema).toBeTruthy();
    expect(rangeResult.current.isRange(regularSchema)).toBe(false);

    // Test range schema operations
    const rangeSchema = approvedSchemas.find(s => s.name === 'RangeSchema');
    expect(rangeSchema).toBeTruthy();
    expect(rangeResult.current.isRange(rangeSchema)).toBe(true);

    // Test range schema specific operations
    const rangeKey = rangeResult.current.rangeProps.getRangeKey(rangeSchema);
    expect(rangeKey).toBe('timestamp');

    const nonRangeFields = rangeResult.current.rangeProps.getNonRangeKeyFields(rangeSchema);
    expect(nonRangeFields).toHaveProperty('value');
    expect(nonRangeFields).not.toHaveProperty('timestamp');
  });

  it('should integrate useApprovedSchemas with useFormValidation for schema validation', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: validationResult } = renderHook(() => useFormValidation());

    // Wait for schemas to load
    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const approvedSchemas = schemasResult.current.approvedSchemas;

    // Test schema approval validation
    const schemaApprovedRule = validationResult.current.createValidationRules.schemaApproved(
      approvedSchemas,
      'Schema must be approved'
    );

    // Should pass for approved schema
    const validationError1 = validationResult.current.validate(
      'selectedSchema',
      'RegularSchema',
      [schemaApprovedRule]
    );
    expect(validationError1).toBe(null);

    // Should fail for non-existent schema
    const validationError2 = validationResult.current.validate(
      'selectedSchema',
      'NonExistentSchema',
      [schemaApprovedRule]
    );
    expect(validationError2).toBe('Schema must be approved');
  });

  it('should integrate all three hooks for complete range schema mutation workflow', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: rangeResult } = renderHook(() => useRangeSchema());
    const { result: validationResult } = renderHook(() => useFormValidation());

    // Wait for schemas to load
    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const rangeSchema = schemasResult.current.approvedSchemas.find(s => s.name === 'RangeSchema');
    expect(rangeSchema).toBeTruthy();

    // Validate that we have an approved range schema
    expect(schemasResult.current.isSchemaApproved('RangeSchema')).toBe(true);
    expect(rangeResult.current.isRange(rangeSchema)).toBe(true);

    // Validate range key
    const rangeKeyError = rangeResult.current.rangeProps.validateRangeKey('test_key_123', true);
    expect(rangeKeyError).toBe(null);

    // Create range mutation
    const fieldData = { value: 42.5 };
    const mutation = rangeResult.current.rangeProps.formatRangeMutation(
      rangeSchema,
      'Create',
      'test_key_123',
      fieldData
    );

    expect(mutation.type).toBe('mutation');
    expect(mutation.schema).toBe('RangeSchema');
    expect(mutation.mutation_type).toBe('create');
    expect(mutation.data.timestamp).toBe('test_key_123');
    expect(mutation.data.value).toEqual({ value: 42.5 });

    // Validate form data using form validation hook
    const formData = {
      selectedSchema: 'RangeSchema',
      rangeKey: 'test_key_123',
      value: 42.5
    };

    const validationRules = [
      validationResult.current.createValidationRules.required('Required field'),
      validationResult.current.createValidationRules.schemaApproved(
        schemasResult.current.approvedSchemas,
        'Schema must be approved'
      )
    ];

    act(() => {
      validationResult.current.validate('selectedSchema', formData.selectedSchema, validationRules);
      validationResult.current.validate('rangeKey', formData.rangeKey, [
        validationResult.current.createValidationRules.required('Range key required')
      ]);
    });

    expect(validationResult.current.isFormValid()).toBe(true);
  });

  it('should handle SCHEMA-002 compliance across all hooks', async () => {
    // Mock with mixed schema states
    fetch
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: ['ApprovedSchema', 'AvailableSchema', 'BlockedSchema'] })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({ 
          data: { 
            'ApprovedSchema': 'approved',
            'AvailableSchema': 'available',
            'BlockedSchema': 'blocked'
          } 
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'ApprovedSchema',
          fields: { field1: { field_type: 'String' } },
          schema_type: { Standard: {} }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'AvailableSchema',
          fields: { field1: { field_type: 'String' } },
          schema_type: { Standard: {} }
        })
      })
      .mockResolvedValueOnce({
        ok: true,
        json: async () => ({
          name: 'BlockedSchema',
          fields: { field1: { field_type: 'String' } },
          schema_type: { Standard: {} }
        })
      });

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: validationResult } = renderHook(() => useFormValidation());

    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    // Only approved schemas should be returned (SCHEMA-002)
    expect(schemasResult.current.approvedSchemas).toHaveLength(1);
    expect(schemasResult.current.approvedSchemas[0].name).toBe('ApprovedSchema');

    // All schemas should be available for inspection
    expect(schemasResult.current.allSchemas).toHaveLength(3);

    // Schema approval validation should enforce SCHEMA-002
    expect(schemasResult.current.isSchemaApproved('ApprovedSchema')).toBe(true);
    expect(schemasResult.current.isSchemaApproved('AvailableSchema')).toBe(false);
    expect(schemasResult.current.isSchemaApproved('BlockedSchema')).toBe(false);

    // Form validation should respect SCHEMA-002
    const schemaRule = validationResult.current.createValidationRules.schemaApproved(
      schemasResult.current.allSchemas,
      'Only approved schemas allowed'
    );

    expect(validationResult.current.validate('schema', 'ApprovedSchema', [schemaRule])).toBe(null);
    expect(validationResult.current.validate('schema', 'AvailableSchema', [schemaRule])).toBe('Only approved schemas allowed');
    expect(validationResult.current.validate('schema', 'BlockedSchema', [schemaRule])).toBe('Only approved schemas allowed');
  });

  it('should handle error states across integrated hooks', async () => {
    // Mock fetch failure
    fetch.mockRejectedValue(new Error('Network error'));

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: validationResult } = renderHook(() => useFormValidation());

    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    expect(schemasResult.current.error).toContain('Failed to fetch schemas');
    expect(schemasResult.current.approvedSchemas).toEqual([]);

    // Validation should handle empty schemas gracefully
    const schemaRule = validationResult.current.createValidationRules.schemaApproved(
      schemasResult.current.approvedSchemas,
      'Schema must be approved'
    );

    const error = validationResult.current.validate('schema', 'AnySchema', [schemaRule]);
    expect(error).toBe('Schema must be approved');
  });

  it('should handle range schema query formatting with approval validation', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: rangeResult } = renderHook(() => useRangeSchema());

    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const rangeSchema = schemasResult.current.approvedSchemas.find(s => s.name === 'RangeSchema');
    
    // Ensure schema is approved before using for queries (SCHEMA-002)
    expect(schemasResult.current.isSchemaApproved('RangeSchema')).toBe(true);

    // Format range query
    const query = rangeResult.current.rangeProps.formatRangeQuery(
      rangeSchema,
      ['value'],
      'filter_key_123'
    );

    expect(query.type).toBe('query');
    expect(query.schema).toBe('RangeSchema');
    expect(query.fields).toEqual(['value']);
    expect(query.range_filter).toEqual({ Key: 'filter_key_123' });
  });

  it('should provide comprehensive schema information through integrated hooks', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: rangeResult } = renderHook(() => useRangeSchema());

    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const allSchemas = schemasResult.current.allSchemas;
    
    allSchemas.forEach(schema => {
      // Get schema by name
      const retrievedSchema = schemasResult.current.getSchemaByName(schema.name);
      expect(retrievedSchema).toEqual(schema);

      // Check approval status
      const isApproved = schemasResult.current.isSchemaApproved(schema.name);
      expect(typeof isApproved).toBe('boolean');

      // Get range information
      const isRange = rangeResult.current.isRange(schema);
      const rangeInfo = rangeResult.current.rangeProps.getRangeSchemaInfo(schema);

      if (isRange) {
        expect(rangeInfo).toBeTruthy();
        expect(rangeInfo.isRangeSchema).toBe(true);
        expect(typeof rangeInfo.rangeKey).toBe('string');
      } else {
        expect(rangeInfo).toBe(null);
      }
    });
  });

  it('should support complex validation scenarios with multiple hooks', async () => {
    mockApprovedSchemasResponse();

    const { result: schemasResult } = await renderHookWithRedux(() => useApprovedSchemas());
    const { result: rangeResult } = renderHook(() => useRangeSchema());
    const { result: validationResult } = renderHook(() => useFormValidation());

    await waitFor(() => {
      expect(schemasResult.current.isLoading).toBe(false);
    });

    const rangeSchema = schemasResult.current.approvedSchemas.find(s => s.name === 'RangeSchema');

    // Complex validation scenario: validate range mutation form
    const formData = {
      selectedSchema: 'RangeSchema',
      mutationType: 'Create',
      rangeKey: 'test_key',
      value: '123.45'
    };

    act(() => {
      // Validate schema selection
      validationResult.current.validate('selectedSchema', formData.selectedSchema, [
        validationResult.current.createValidationRules.required('Schema is required'),
        validationResult.current.createValidationRules.schemaApproved(
          schemasResult.current.allSchemas,
          'Schema must be approved'
        )
      ]);

      // Validate range key
      const rangeKeyError = rangeResult.current.rangeProps.validateRangeKey(formData.rangeKey, true);
      if (rangeKeyError) {
        validationResult.current.setFieldError('rangeKey', rangeKeyError);
      }

      // Validate mutation type
      validationResult.current.validate('mutationType', formData.mutationType, [
        validationResult.current.createValidationRules.required('Mutation type is required'),
        validationResult.current.createValidationRules.custom(
          (value) => ['Create', 'Update', 'Delete'].includes(value) ? null : 'Invalid mutation type',
          'Must be a valid mutation type'
        )
      ]);

      // Validate value field
      validationResult.current.validate('value', formData.value, [
        validationResult.current.createValidationRules.required('Value is required'),
        validationResult.current.createValidationRules.type('number', 'Value must be a number')
      ]);
    });

    expect(validationResult.current.isFormValid()).toBe(true);

    // Create the mutation if validation passes
    if (validationResult.current.isFormValid()) {
      const mutation = rangeResult.current.rangeProps.formatRangeMutation(
        rangeSchema,
        formData.mutationType,
        formData.rangeKey,
        { value: parseFloat(formData.value) }
      );

      expect(mutation.type).toBe('mutation');
      expect(mutation.schema).toBe('RangeSchema');
      expect(mutation.data.timestamp).toBe('test_key');
      expect(mutation.data.value).toEqual({ value: 123.45 });
    }
  });
});