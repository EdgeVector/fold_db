/**
 * @fileoverview Schema Test Fixtures
 * 
 * Provides comprehensive test fixtures for schema objects, including
 * standard schemas, range schemas, and various schema states for
 * consistent testing across the application.
 * 
 * TASK-006: Testing Enhancement - Created schema test fixtures
 * 
 * @module schemaFixtures
 * @since 2.0.0
 */

import { SCHEMA_STATES } from '../../constants/schemas';

// ============================================================================
// BASIC SCHEMA FIXTURES
// ============================================================================

/**
 * Basic approved schema for general testing
 */
export const basicApprovedSchema = {
  name: 'user_profiles',
  state: SCHEMA_STATES.APPROVED,
  fields: ['id', 'name', 'email', 'age', 'active'],
  schema_type: { Single: {} },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Basic available schema for testing state transitions
 */
export const basicAvailableSchema = {
  name: 'product_catalog',
  state: SCHEMA_STATES.AVAILABLE,
  fields: ['product_id', 'name', 'price', 'category', 'in_stock'],
  schema_type: { Single: {} },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Basic blocked schema for testing access restrictions
 */
export const basicBlockedSchema = {
  name: 'legacy_orders',
  state: SCHEMA_STATES.BLOCKED,
  fields: ['order_id', 'customer_id', 'total'],
  schema_type: { Single: {} },
  created_at: '2025-06-23T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

// ============================================================================
// RANGE SCHEMA FIXTURES
// ============================================================================

/**
 * Time series range schema for testing range operations
 */
export const timeSeriesRangeSchema = {
  name: 'time_series_data',
  state: SCHEMA_STATES.APPROVED,
  fields: ['timestamp', 'value', 'metadata'],
  key: { range_field: 'timestamp' },
  schema_type: {
    Range: { 
      range_key: 'timestamp'
    }
  },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * User activity range schema for testing complex range operations
 */
export const userActivityRangeSchema = {
  name: 'user_activity',
  state: SCHEMA_STATES.APPROVED,
  fields: ['user_id', 'activity_type', 'session_data', 'metrics'],
  key: { range_field: 'user_id' },
  schema_type: {
    Range: { 
      range_key: 'user_id'
    }
  },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Available range schema for testing state restrictions
 */
export const availableRangeSchema = {
  name: 'sensor_readings',
  state: SCHEMA_STATES.AVAILABLE,
  fields: ['sensor_id', 'reading_value', 'calibration_data'],
  key: { range_field: 'sensor_id' },
  schema_type: {
    Range: { 
      range_key: 'sensor_id'
    }
  },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

// ============================================================================
// COMPLEX SCHEMA FIXTURES
// ============================================================================

/**
 * Complex schema with mixed field types
 */
export const complexMixedSchema = {
  name: 'analytics_events',
  state: SCHEMA_STATES.APPROVED,
  fields: ['event_id', 'user_id', 'event_type', 'timestamp', 'properties', 'session_duration', 'page_views', 'is_conversion', 'is_bounce'],
  schema_type: { Single: {} },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Schema with minimal fields for edge case testing
 */
export const minimalSchema = {
  name: 'simple_counter',
  state: SCHEMA_STATES.APPROVED,
  fields: ['count'],
  schema_type: { Single: {} },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Schema with only string fields
 */
export const stringOnlySchema = {
  name: 'text_content',
  state: SCHEMA_STATES.APPROVED,
  fields: ['title', 'body', 'author', 'tags'],
  schema_type: { Single: {} },
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

// ============================================================================
// SCHEMA COLLECTIONS
// ============================================================================

/**
 * Collection of all approved schemas for testing
 */
export const approvedSchemas = [
  basicApprovedSchema,
  timeSeriesRangeSchema,
  userActivityRangeSchema,
  complexMixedSchema,
  minimalSchema,
  stringOnlySchema
];

/**
 * Collection of all available schemas for testing
 */
export const availableSchemas = [
  basicAvailableSchema,
  availableRangeSchema
];

/**
 * Collection of all blocked schemas for testing
 */
export const blockedSchemas = [
  basicBlockedSchema
];

/**
 * Collection of all schemas regardless of state
 */
export const allSchemas = [
  ...approvedSchemas,
  ...availableSchemas,
  ...blockedSchemas
];

/**
 * Collection of all range schemas
 */
export const rangeSchemas = [
  timeSeriesRangeSchema,
  userActivityRangeSchema,
  availableRangeSchema
];

/**
 * Collection of all standard (non-range) schemas
 */
export const standardSchemas = allSchemas.filter(
  schema => !rangeSchemas.includes(schema)
);

// ============================================================================
// SCHEMA STATE MAPPINGS
// ============================================================================

/**
 * Mapping of schema names to their states
 */
export const schemaStateMap = allSchemas.reduce((map, schema) => {
  map[schema.name] = schema.state;
  return map;
}, {});

/**
 * Mapping of schema names to their full objects
 */
export const schemaObjectMap = allSchemas.reduce((map, schema) => {
  map[schema.name] = schema;
  return map;
}, {});

/**
 * List of schema names only
 */
export const schemaNames = allSchemas.map(schema => schema.name);

/**
 * List of approved schema names only
 */
export const approvedSchemaNames = approvedSchemas.map(schema => schema.name);

/**
 * List of range schema names only
 */
export const rangeSchemaNames = rangeSchemas.map(schema => schema.name);

// ============================================================================
// FACTORY FUNCTIONS
// ============================================================================

/**
 * Creates a custom schema fixture with specified properties
 * 
 * @param {Object} overrides - Properties to override in base schema
 * @param {string} baseType - Base schema type ('standard' or 'range')
 * @returns {Object} Custom schema fixture
 */
export const createCustomSchema = (overrides = {}, baseType = 'standard') => {
  const baseSchema = baseType === 'range' ? timeSeriesRangeSchema : basicApprovedSchema;
  
  return {
    ...baseSchema,
    name: `custom_schema_${Math.random().toString(36).substr(2, 9)}`,
    ...overrides,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString()
  };
};

/**
 * Creates a schema with specific state
 * 
 * @param {string} state - Schema state
 * @param {Object} overrides - Additional properties to override
 * @returns {Object} Schema fixture with specified state
 */
export const createSchemaWithState = (state, overrides = {}) => {
  return createCustomSchema({
    state,
    ...overrides
  });
};

/**
 * Creates a range schema with custom range key
 * 
 * @param {string} rangeKey - Name of the range key field
 * @param {Object} overrides - Additional properties to override
 * @returns {Object} Range schema fixture
 */
export const createRangeSchemaWithKey = (rangeKey, overrides = {}) => {
  return createCustomSchema({
    fields: [rangeKey, 'value'],
    key: { range_field: rangeKey },
    schema_type: 'Range',
    ...overrides
  }, 'range');
};

/**
 * Creates a schema list with mixed states for testing
 * 
 * @param {number} count - Number of schemas to create
 * @param {Array} states - Array of states to cycle through
 * @returns {Array} Array of schema fixtures
 */
export const createMixedSchemaList = (count = 6, states = Object.values(SCHEMA_STATES)) => {
  return Array.from({ length: count }, (_, index) => {
    const state = states[index % states.length];
    const isRange = index % 3 === 0; // Every third schema is a range schema
    
    return createCustomSchema({
      name: `mixed_schema_${index}`,
      state,
      ...(isRange && {
        fields: ['range_key', 'data'],
        key: { range_field: 'range_key' },
        schema_type: 'Range'
      })
    }, isRange ? 'range' : 'standard');
  });
};

// ============================================================================
// VALIDATION HELPERS
// ============================================================================

/**
 * Validates that a schema has the expected structure
 * 
 * @param {Object} schema - Schema to validate
 * @returns {boolean} True if schema is valid
 */
export const isValidSchemaFixture = (schema) => {
  const requiredFields = ['name', 'state', 'fields', 'schema_type'];
  const validStates = Object.values(SCHEMA_STATES);
  
  return (
    requiredFields.every(field => field in schema) &&
    validStates.includes(schema.state) &&
    Array.isArray(schema.fields) &&
    schema.fields.length > 0
  );
};

/**
 * Validates that a schema is a proper range schema
 * 
 * @param {Object} schema - Schema to validate
 * @returns {boolean} True if schema is a valid range schema
 */
export const isValidRangeSchemaFixture = (schema) => {
  if (!isValidSchemaFixture(schema)) return false;
  
  const hasRangeType = schema.schema_type === 'Range';
  const hasKeyConfig = schema.key?.range_field;
  
  return hasRangeType && hasKeyConfig;
};

// Export all fixtures and utilities
export default {
  // Basic schemas
  basicApprovedSchema,
  basicAvailableSchema,
  basicBlockedSchema,
  
  // Range schemas
  timeSeriesRangeSchema,
  userActivityRangeSchema,
  availableRangeSchema,
  
  // Complex schemas
  complexMixedSchema,
  minimalSchema,
  stringOnlySchema,
  
  // Collections
  approvedSchemas,
  availableSchemas,
  blockedSchemas,
  allSchemas,
  rangeSchemas,
  standardSchemas,
  
  // Mappings
  schemaStateMap,
  schemaObjectMap,
  schemaNames,
  approvedSchemaNames,
  rangeSchemaNames,
  
  // Factory functions
  createCustomSchema,
  createSchemaWithState,
  createRangeSchemaWithKey,
  createMixedSchemaList,
  
  // Validation helpers
  isValidSchemaFixture,
  isValidRangeSchemaFixture
};