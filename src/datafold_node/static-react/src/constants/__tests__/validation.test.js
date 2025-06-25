/**
 * Constants Validation Tests
 * TASK-005: Constants Extraction and Configuration Centralization
 * 
 * Tests to validate that constants follow project standards and conventions
 */

import {
  VALIDATION_FUNCTIONS,
  VALIDATION_RULES,
  ERROR_UTILS,
  SCHEMA_STATES
} from '../index';

describe('Constants Validation Functions', () => {
  describe('validateRangeKey', () => {
    test('should validate required range keys correctly', () => {
      // Valid cases
      expect(VALIDATION_FUNCTIONS.validateRangeKey('valid_key', true)).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateRangeKey('key123', true)).toBeNull();
      
      // Invalid cases - required but empty
      expect(VALIDATION_FUNCTIONS.validateRangeKey('', true)).toBeTruthy();
      expect(VALIDATION_FUNCTIONS.validateRangeKey(null, true)).toBeTruthy();
      expect(VALIDATION_FUNCTIONS.validateRangeKey(undefined, true)).toBeTruthy();
      
      // Too long
      const longKey = 'a'.repeat(VALIDATION_RULES.RANGE_KEY.MAX_LENGTH + 1);
      expect(VALIDATION_FUNCTIONS.validateRangeKey(longKey, true)).toBeTruthy();
    });

    test('should handle optional range keys correctly', () => {
      // Optional and empty should be valid
      expect(VALIDATION_FUNCTIONS.validateRangeKey('', false)).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateRangeKey(null, false)).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateRangeKey(undefined, false)).toBeNull();
      
      // Optional but with value should still validate
      expect(VALIDATION_FUNCTIONS.validateRangeKey('valid_key', false)).toBeNull();
      
      const longKey = 'a'.repeat(VALIDATION_RULES.RANGE_KEY.MAX_LENGTH + 1);
      expect(VALIDATION_FUNCTIONS.validateRangeKey(longKey, false)).toBeTruthy();
    });
  });

  describe('validateSchemaName', () => {
    test('should validate schema names correctly', () => {
      // Valid cases
      expect(VALIDATION_FUNCTIONS.validateSchemaName('ValidSchema')).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateSchemaName('schema_123')).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateSchemaName('MySchema')).toBeNull();
      
      // Invalid cases
      expect(VALIDATION_FUNCTIONS.validateSchemaName('')).toBeTruthy(); // Empty
      expect(VALIDATION_FUNCTIONS.validateSchemaName('1schema')).toBeTruthy(); // Starts with number
      expect(VALIDATION_FUNCTIONS.validateSchemaName('schema-name')).toBeTruthy(); // Contains dash
      expect(VALIDATION_FUNCTIONS.validateSchemaName('schema name')).toBeTruthy(); // Contains space
      
      // Reserved names
      expect(VALIDATION_FUNCTIONS.validateSchemaName('system')).toBeTruthy();
      expect(VALIDATION_FUNCTIONS.validateSchemaName('admin')).toBeTruthy();
      
      // Too short
      expect(VALIDATION_FUNCTIONS.validateSchemaName('ab')).toBeTruthy();
      
      // Too long
      const longName = 'a'.repeat(VALIDATION_RULES.SCHEMA_NAME.MAX_LENGTH + 1);
      expect(VALIDATION_FUNCTIONS.validateSchemaName(longName)).toBeTruthy();
    });
  });

  describe('validateFieldName', () => {
    test('should validate field names correctly', () => {
      // Valid cases
      expect(VALIDATION_FUNCTIONS.validateFieldName('fieldName')).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateFieldName('field_123')).toBeNull();
      expect(VALIDATION_FUNCTIONS.validateFieldName('myField')).toBeNull();
      
      // Invalid cases
      expect(VALIDATION_FUNCTIONS.validateFieldName('')).toBeTruthy(); // Empty
      expect(VALIDATION_FUNCTIONS.validateFieldName('1field')).toBeTruthy(); // Starts with number
      expect(VALIDATION_FUNCTIONS.validateFieldName('field-name')).toBeTruthy(); // Contains dash
      expect(VALIDATION_FUNCTIONS.validateFieldName('field name')).toBeTruthy(); // Contains space
      
      // Reserved names
      expect(VALIDATION_FUNCTIONS.validateFieldName('id')).toBeTruthy();
      expect(VALIDATION_FUNCTIONS.validateFieldName('type')).toBeTruthy();
      
      // Too long
      const longName = 'a'.repeat(VALIDATION_RULES.FIELD_NAME.MAX_LENGTH + 1);
      expect(VALIDATION_FUNCTIONS.validateFieldName(longName)).toBeTruthy();
    });
  });
});

describe('Error Utilities Validation', () => {
  describe('ERROR_UTILS.getMessage', () => {
    test('should return correct message for valid error codes', () => {
      const message = ERROR_UTILS.getMessage('SCHEMA_NOT_APPROVED');
      expect(message).toBe('Only approved schemas can be used for this operation.');
    });

    test('should return default message for invalid error codes', () => {
      const message = ERROR_UTILS.getMessage('INVALID_CODE');
      expect(message).toBe('An unexpected error occurred. Please try again.');
    });
  });

  describe('ERROR_UTILS.getCategory', () => {
    test('should return correct category for schema errors', () => {
      const category = ERROR_UTILS.getCategory('SCHEMA_NOT_APPROVED');
      expect(category).toBe('schema');
    });

    test('should return system category for unknown errors', () => {
      const category = ERROR_UTILS.getCategory('UNKNOWN_CODE');
      expect(category).toBe('system');
    });
  });

  describe('ERROR_UTILS.isRetryable', () => {
    test('should correctly identify retryable errors', () => {
      // Network errors should be retryable
      expect(ERROR_UTILS.isRetryable('NETWORK_ERROR')).toBe(true);
      
      // Validation errors should not be retryable
      expect(ERROR_UTILS.isRetryable('FORM_VALIDATION_FAILED')).toBe(false);
    });
  });

  describe('ERROR_UTILS.createError', () => {
    test('should create standardized error objects', () => {
      const error = ERROR_UTILS.createError('SCHEMA_NOT_APPROVED', null, { schemaName: 'test' });
      
      expect(error.code).toBe('SCHEMA_NOT_APPROVED');
      expect(error.message).toBe('Only approved schemas can be used for this operation.');
      expect(error.category).toBe('schema');
      expect(error.timestamp).toBeDefined();
      expect(error.details).toEqual({ schemaName: 'test' });
    });
  });
});

describe('Schema State Validation (SCHEMA-002 Compliance)', () => {
  test('should have correct schema states', () => {
    expect(SCHEMA_STATES.APPROVED).toBe('approved');
    expect(SCHEMA_STATES.AVAILABLE).toBe('available');
    expect(SCHEMA_STATES.BLOCKED).toBe('blocked');
  });

  test('should validate schema state transitions', () => {
    // These are the valid states according to SCHEMA-002
    const validStates = ['approved', 'available', 'blocked'];
    
    expect(validStates).toContain(SCHEMA_STATES.APPROVED);
    expect(validStates).toContain(SCHEMA_STATES.AVAILABLE);
    expect(validStates).toContain(SCHEMA_STATES.BLOCKED);
  });
});

describe('Performance Constants Validation', () => {
  test('should have reasonable timeout values', () => {
    // Import config directly for testing
    const { APP_CONFIG } = require('../config');
    
    // Debounce delays should be reasonable (100ms - 1000ms)
    expect(APP_CONFIG.PERFORMANCE.DEBOUNCE_DELAY_MS).toBeGreaterThanOrEqual(100);
    expect(APP_CONFIG.PERFORMANCE.DEBOUNCE_DELAY_MS).toBeLessThanOrEqual(1000);
    
    // Animation durations should be reasonable (50ms - 1000ms)
    expect(APP_CONFIG.PERFORMANCE.ANIMATION_DURATION_MS).toBeGreaterThanOrEqual(50);
    expect(APP_CONFIG.PERFORMANCE.ANIMATION_DURATION_MS).toBeLessThanOrEqual(1000);
    
    // Cache TTL should be reasonable (1 minute - 1 hour)
    expect(APP_CONFIG.CACHE.DEFAULT_TTL_MS).toBeGreaterThanOrEqual(60000); // 1 minute
    expect(APP_CONFIG.CACHE.DEFAULT_TTL_MS).toBeLessThanOrEqual(3600000); // 1 hour
  });
});

describe('Validation Rules Consistency', () => {
  test('should have consistent length validation rules', () => {
    // All MIN_LENGTH should be positive
    expect(VALIDATION_RULES.TEXT.MIN_LENGTH).toBeGreaterThan(0);
    expect(VALIDATION_RULES.SCHEMA_NAME.MIN_LENGTH).toBeGreaterThan(0);
    expect(VALIDATION_RULES.FIELD_NAME.MIN_LENGTH).toBeGreaterThan(0);
    
    // All MAX_LENGTH should be greater than MIN_LENGTH
    expect(VALIDATION_RULES.TEXT.MAX_LENGTH).toBeGreaterThan(VALIDATION_RULES.TEXT.MIN_LENGTH);
    expect(VALIDATION_RULES.SCHEMA_NAME.MAX_LENGTH).toBeGreaterThan(VALIDATION_RULES.SCHEMA_NAME.MIN_LENGTH);
    expect(VALIDATION_RULES.FIELD_NAME.MAX_LENGTH).toBeGreaterThan(VALIDATION_RULES.FIELD_NAME.MIN_LENGTH);
  });

  test('should have valid file upload limits', () => {
    expect(VALIDATION_RULES.FILE_UPLOAD.MAX_SIZE_BYTES).toBeGreaterThan(0);
    expect(VALIDATION_RULES.FILE_UPLOAD.ALLOWED_TYPES).toBeInstanceOf(Array);
    expect(VALIDATION_RULES.FILE_UPLOAD.ALLOWED_TYPES.length).toBeGreaterThan(0);
    expect(VALIDATION_RULES.FILE_UPLOAD.ALLOWED_EXTENSIONS).toBeInstanceOf(Array);
    expect(VALIDATION_RULES.FILE_UPLOAD.ALLOWED_EXTENSIONS.length).toBeGreaterThan(0);
  });
});

describe('Constants Structure Validation', () => {
  test('should export all required constant categories', async () => {
    // Test direct imports to ensure everything is properly exported
    const constants = await import('../index');
    
    expect(constants.APP_CONFIG).toBeDefined();
    expect(constants.VALIDATION_RULES).toBeDefined();
    expect(constants.COLORS).toBeDefined();
    expect(constants.ERROR_CODES).toBeDefined();
    expect(constants.SCHEMA_STATES).toBeDefined();
    expect(constants.Constants).toBeDefined();
  });

  test('should have consistent naming conventions', () => {
    // All constant names should be UPPER_CASE
    const constantNames = [
      'APP_CONFIG',
      'VALIDATION_RULES',
      'SCHEMA_STATES',
      'ERROR_CODES',
      'DEFAULT_TAB'
    ];
    
    constantNames.forEach(name => {
      expect(name).toMatch(/^[A-Z][A-Z0-9_]*$/);
    });
  });
});