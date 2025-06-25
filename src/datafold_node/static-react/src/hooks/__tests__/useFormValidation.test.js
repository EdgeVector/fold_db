import { renderHook, act, waitFor } from '@testing-library/react';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFormValidation } from '../useFormValidation.js';
import { 
  FORM_VALIDATION_DEBOUNCE_MS,
  VALIDATION_MESSAGES,
  FIELD_TYPES,
  SCHEMA_STATES
} from '../../constants/schemas.js';

describe('useFormValidation Hook', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  const createMockSchema = () => ({
    name: 'TestSchema',
    fields: {
      username: { field_type: 'String' },
      age: { field_type: 'Number' },
      active: { field_type: 'Boolean' },
      range_field: { field_type: 'Range' }
    }
  });

  const createMockSchemas = () => [
    { name: 'ApprovedSchema', state: 'approved' },
    { name: 'AvailableSchema', state: 'available' },
    { name: 'BlockedSchema', state: 'blocked' }
  ];

  it('should initialize with empty errors', () => {
    const { result } = renderHook(() => useFormValidation());

    expect(result.current.errors).toEqual({});
    expect(result.current.isFormValid()).toBe(true);
  });

  it('should validate required fields correctly', () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = { type: 'required', value: true };

    // Valid values
    expect(result.current.validate('field1', 'value', [requiredRule])).toBe(null);
    expect(result.current.validate('field2', 123, [requiredRule])).toBe(null);
    expect(result.current.validate('field3', false, [requiredRule])).toBe(null);

    // Invalid values
    expect(result.current.validate('field4', '', [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    expect(result.current.validate('field5', '   ', [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    expect(result.current.validate('field6', null, [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    expect(result.current.validate('field7', undefined, [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    expect(result.current.validate('field8', [], [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    expect(result.current.validate('field9', {}, [requiredRule])).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
  });

  it('should validate field types correctly', () => {
    const { result } = renderHook(() => useFormValidation());

    // String validation
    const stringRule = { type: 'type', value: FIELD_TYPES.STRING };
    expect(result.current.validate('field1', 'valid string', [stringRule])).toBe(null);
    expect(result.current.validate('field2', 123, [stringRule])).toContain('Expected string, got number');

    // Number validation
    const numberRule = { type: 'type', value: FIELD_TYPES.NUMBER };
    expect(result.current.validate('field3', 123, [numberRule])).toBe(null);
    expect(result.current.validate('field4', '123', [numberRule])).toBe(null); // String numbers are valid
    expect(result.current.validate('field5', 'abc', [numberRule])).toContain('Expected number');

    // Boolean validation
    const booleanRule = { type: 'type', value: FIELD_TYPES.BOOLEAN };
    expect(result.current.validate('field6', true, [booleanRule])).toBe(null);
    expect(result.current.validate('field7', 'true', [booleanRule])).toBe(null);
    expect(result.current.validate('field8', 'false', [booleanRule])).toBe(null);
    expect(result.current.validate('field9', 'invalid', [booleanRule])).toContain('Expected boolean');
  });

  it('should handle custom validation rules', () => {
    const { result } = renderHook(() => useFormValidation());

    const customValidator = (value) => {
      if (value && value.includes('forbidden')) {
        return 'Value contains forbidden word';
      }
      return null;
    };

    const customRule = { type: 'custom', validator: customValidator };

    expect(result.current.validate('field1', 'safe text', [customRule])).toBe(null);
    expect(result.current.validate('field2', 'forbidden word', [customRule])).toBe('Value contains forbidden word');
  });

  it('should validate schema approval status', () => {
    const { result } = renderHook(() => useFormValidation());
    const schemas = createMockSchemas();

    const schemaApprovedRule = { type: 'schema_approved', value: true, schemas };

    expect(result.current.validate('schema1', 'ApprovedSchema', [schemaApprovedRule])).toBe(null);
    expect(result.current.validate('schema2', 'AvailableSchema', [schemaApprovedRule])).toBe(VALIDATION_MESSAGES.SCHEMA_NOT_APPROVED);
    expect(result.current.validate('schema3', 'NonExistentSchema', [schemaApprovedRule])).toBe(VALIDATION_MESSAGES.SCHEMA_NOT_APPROVED);
  });

  it('should handle debounced validation', async () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = { type: 'required', value: true };

    // Start debounced validation
    act(() => {
      result.current.validate('field1', '', [requiredRule], true);
    });

    // Should not have error immediately
    expect(result.current.errors.field1).toBeUndefined();

    // Fast-forward time to trigger debounce
    act(() => {
      vi.advanceTimersByTime(FORM_VALIDATION_DEBOUNCE_MS);
    });

    await waitFor(() => {
      expect(result.current.errors.field1).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);
    });
  });

  it('should cancel previous debounced validation when new validation starts', async () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = { type: 'required', value: true };

    // Start first debounced validation
    act(() => {
      result.current.validate('field1', '', [requiredRule], true);
    });

    // Start second debounced validation before first completes
    act(() => {
      vi.advanceTimersByTime(FORM_VALIDATION_DEBOUNCE_MS / 2);
      result.current.validate('field1', 'valid', [requiredRule], true);
    });

    // Complete the debounce period
    act(() => {
      vi.advanceTimersByTime(FORM_VALIDATION_DEBOUNCE_MS);
    });

    await waitFor(() => {
      expect(result.current.errors.field1).toBeUndefined(); // Should be cleared due to valid value
    });
  });

  it('should validate entire forms against schemas', () => {
    const { result } = renderHook(() => useFormValidation());
    const schema = createMockSchema();

    const formData = {
      username: 'john_doe',
      age: 25,
      active: true,
      range_field: 'some_range_value'
    };

    const validationConfig = {
      requiredFields: ['username', 'age'],
      customRules: {
        username: [{ type: 'custom', validator: (value) => value.length < 3 ? 'Too short' : null }]
      }
    };

    const errors = result.current.validateForm(formData, schema, validationConfig);

    expect(Object.keys(errors)).toHaveLength(0); // No errors expected
  });

  it('should detect form validation errors', () => {
    const { result } = renderHook(() => useFormValidation());
    const schema = createMockSchema();

    const formData = {
      username: '', // Required but empty
      age: 'not_a_number', // Invalid type
      active: true,
      range_field: 'valid'
    };

    const validationConfig = {
      requiredFields: ['username']
    };

    act(() => {
      result.current.validateForm(formData, schema, validationConfig);
    });

    expect(result.current.isFormValid()).toBe(false);
    expect(Object.keys(result.current.errors).length).toBeGreaterThan(0);
  });

  it('should provide helper functions for error management', () => {
    const { result } = renderHook(() => useFormValidation());

    // Set some errors manually
    act(() => {
      result.current.setFieldError('field1', 'Error 1');
      result.current.setFieldError('field2', 'Error 2');
    });

    expect(result.current.getFieldError('field1')).toBe('Error 1');
    expect(result.current.getFieldError('field2')).toBe('Error 2');
    expect(result.current.getFieldError('field3')).toBe(null);

    expect(result.current.isFormValid()).toBe(false);

    // Clear errors
    act(() => {
      result.current.clearErrors();
    });

    expect(result.current.errors).toEqual({});
    expect(result.current.isFormValid()).toBe(true);
  });

  it('should provide validation rule creators', () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = result.current.createValidationRules.required('Custom required message');
    expect(requiredRule.type).toBe('required');
    expect(requiredRule.value).toBe(true);
    expect(requiredRule.message).toBe('Custom required message');

    const typeRule = result.current.createValidationRules.type(FIELD_TYPES.STRING, 'Custom type message');
    expect(typeRule.type).toBe('type');
    expect(typeRule.value).toBe(FIELD_TYPES.STRING);
    expect(typeRule.message).toBe('Custom type message');

    const customValidator = (value) => value === 'test' ? null : 'Not test';
    const customRule = result.current.createValidationRules.custom(customValidator, 'Custom validation message');
    expect(customRule.type).toBe('custom');
    expect(customRule.validator).toBe(customValidator);
    expect(customRule.message).toBe('Custom validation message');

    const schemas = createMockSchemas();
    const schemaRule = result.current.createValidationRules.schemaApproved(schemas, 'Schema not approved');
    expect(schemaRule.type).toBe('schema_approved');
    expect(schemaRule.value).toBe(true);
    expect(schemaRule.schemas).toBe(schemas);
    expect(schemaRule.message).toBe('Schema not approved');
  });

  it('should handle custom error messages in validation rules', () => {
    const { result } = renderHook(() => useFormValidation());

    const customMessage = 'This field is absolutely required!';
    const requiredRule = { type: 'required', value: true, message: customMessage };

    const error = result.current.validate('field1', '', [requiredRule]);
    expect(error).toBe(customMessage);
  });

  it('should skip type validation for empty values', () => {
    const { result } = renderHook(() => useFormValidation());

    const typeRule = { type: 'type', value: FIELD_TYPES.NUMBER };

    // Empty values should not trigger type validation errors
    expect(result.current.validate('field1', '', [typeRule])).toBe(null);
    expect(result.current.validate('field2', null, [typeRule])).toBe(null);
    expect(result.current.validate('field3', undefined, [typeRule])).toBe(null);
  });

  it('should handle schema state normalization correctly', () => {
    const { result } = renderHook(() => useFormValidation());

    const schemas = [
      { name: 'Schema1', state: 'APPROVED' }, // Uppercase
      { name: 'Schema2', state: { toString: () => 'approved' } }, // Object with toString
      { name: 'Schema3', state: 'Available' } // Mixed case, not approved
    ];

    const schemaRule = { type: 'schema_approved', value: true, schemas };

    expect(result.current.validate('field1', 'Schema1', [schemaRule])).toBe(null);
    expect(result.current.validate('field2', 'Schema2', [schemaRule])).toBe(null);
    expect(result.current.validate('field3', 'Schema3', [schemaRule])).toBe(VALIDATION_MESSAGES.SCHEMA_NOT_APPROVED);
  });

  it('should handle unknown validation rule types gracefully', () => {
    const { result } = renderHook(() => useFormValidation());
    const mockConsole = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const unknownRule = { type: 'unknown_type', value: 'test' };

    const error = result.current.validate('field1', 'value', [unknownRule]);
    expect(error).toBe(null); // Should not produce error
    expect(mockConsole).toHaveBeenCalledWith('Unknown validation rule type: unknown_type');

    mockConsole.mockRestore();
  });

  it('should clear debounce timers when clearErrors is called', () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = { type: 'required', value: true };

    // Start debounced validation
    act(() => {
      result.current.validate('field1', '', [requiredRule], true);
    });

    // Clear errors before debounce completes
    act(() => {
      result.current.clearErrors();
      vi.advanceTimersByTime(FORM_VALIDATION_DEBOUNCE_MS);
    });

    // Should not have any errors since timers were cleared
    expect(result.current.errors).toEqual({});
  });

  it('should handle complex form validation scenarios', () => {
    const { result } = renderHook(() => useFormValidation());
    const schema = createMockSchema();

    const formData = {
      username: 'ab', // Too short
      age: 25,
      active: 'invalid_boolean',
      range_field: null // Required
    };

    const validationConfig = {
      requiredFields: ['username', 'range_field'],
      customRules: {
        username: [
          { type: 'custom', validator: (value) => value.length < 3 ? 'Username too short' : null }
        ]
      }
    };

    act(() => {
      const errors = result.current.validateForm(formData, schema, validationConfig);
      expect(Object.keys(errors).length).toBeGreaterThan(0);
    });

    expect(result.current.isFormValid()).toBe(false);
  });

  it('should handle schemas without fields in form validation', () => {
    const { result } = renderHook(() => useFormValidation());

    const emptySchema = { name: 'EmptySchema' };
    const formData = { field1: 'value' };

    const errors = result.current.validateForm(formData, emptySchema);
    expect(errors).toEqual({});
  });

  it('should update errors state correctly during validation', () => {
    const { result } = renderHook(() => useFormValidation());

    const requiredRule = { type: 'required', value: true };

    // Set an error
    act(() => {
      result.current.validate('field1', '', [requiredRule]);
    });

    expect(result.current.errors.field1).toBe(VALIDATION_MESSAGES.FIELD_REQUIRED);

    // Clear the error with valid value
    act(() => {
      result.current.validate('field1', 'valid', [requiredRule]);
    });

    expect(result.current.errors.field1).toBeUndefined();
  });
});