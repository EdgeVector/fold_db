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
  fields: {
    id: { 
      field_type: 'String',
      description: 'Unique user identifier'
    },
    name: { 
      field_type: 'String',
      description: 'User full name'
    },
    email: { 
      field_type: 'String',
      description: 'User email address'
    },
    age: { 
      field_type: 'Number',
      description: 'User age in years'
    },
    active: { 
      field_type: 'Boolean',
      description: 'Whether user is active'
    }
  },
  schema_type: 'Standard',
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Basic available schema for testing state transitions
 */
export const basicAvailableSchema = {
  name: 'product_catalog',
  state: SCHEMA_STATES.AVAILABLE,
  fields: {
    product_id: { 
      field_type: 'String',
      description: 'Unique product identifier'
    },
    name: { 
      field_type: 'String',
      description: 'Product name'
    },
    price: { 
      field_type: 'Number',
      description: 'Product price in cents'
    },
    category: { 
      field_type: 'String',
      description: 'Product category'
    },
    in_stock: { 
      field_type: 'Boolean',
      description: 'Whether product is in stock'
    }
  },
  schema_type: 'Standard',
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Basic blocked schema for testing access restrictions
 */
export const basicBlockedSchema = {
  name: 'legacy_orders',
  state: SCHEMA_STATES.BLOCKED,
  fields: {
    order_id: { 
      field_type: 'String',
      description: 'Legacy order identifier'
    },
    customer_id: { 
      field_type: 'String',
      description: 'Customer identifier'
    },
    total: { 
      field_type: 'Number',
      description: 'Order total amount'
    }
  },
  schema_type: 'Standard',
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
  fields: {
    timestamp: { 
      field_type: 'Range',
      description: 'Timestamp for data point'
    },
    value: { 
      field_type: 'Range',
      description: 'Numeric value at timestamp'
    },
    metadata: { 
      field_type: 'Range',
      description: 'Additional metadata for data point'
    }
  },
  schema_type: {
    Range: { 
      range_key: 'timestamp'
    }
  },
  rangeInfo: {
    isRangeSchema: true,
    rangeField: {
      name: 'timestamp',
      type: 'Range'
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
  fields: {
    user_id: { 
      field_type: 'Range',
      description: 'User identifier for activity'
    },
    activity_type: { 
      field_type: 'Range',
      description: 'Type of user activity'
    },
    session_data: { 
      field_type: 'Range',
      description: 'Session information'
    },
    metrics: { 
      field_type: 'Range',
      description: 'Activity metrics'
    }
  },
  schema_type: {
    Range: { 
      range_key: 'user_id'
    }
  },
  rangeInfo: {
    isRangeSchema: true,
    rangeField: {
      name: 'user_id',
      type: 'Range'
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
  fields: {
    sensor_id: { 
      field_type: 'Range',
      description: 'Sensor identifier'
    },
    reading_value: { 
      field_type: 'Range',
      description: 'Sensor reading value'
    },
    calibration_data: { 
      field_type: 'Range',
      description: 'Sensor calibration information'
    }
  },
  schema_type: {
    Range: { 
      range_key: 'sensor_id'
    }
  },
  rangeInfo: {
    isRangeSchema: true,
    rangeField: {
      name: 'sensor_id',
      type: 'Range'
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
  fields: {
    event_id: { 
      field_type: 'String',
      description: 'Unique event identifier'
    },
    user_id: { 
      field_type: 'String',
      description: 'User who triggered event'
    },
    event_type: { 
      field_type: 'String',
      description: 'Type of analytics event'
    },
    timestamp: { 
      field_type: 'String',
      description: 'ISO timestamp of event'
    },
    properties: { 
      field_type: 'String',
      description: 'JSON string of event properties'
    },
    session_duration: { 
      field_type: 'Number',
      description: 'Duration of user session in seconds'
    },
    page_views: { 
      field_type: 'Number',
      description: 'Number of page views in session'
    },
    is_conversion: { 
      field_type: 'Boolean',
      description: 'Whether event represents a conversion'
    },
    is_bounce: { 
      field_type: 'Boolean',
      description: 'Whether event represents a bounce'
    }
  },
  schema_type: 'Standard',
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Schema with minimal fields for edge case testing
 */
export const minimalSchema = {
  name: 'simple_counter',
  state: SCHEMA_STATES.APPROVED,
  fields: {
    count: { 
      field_type: 'Number',
      description: 'Simple counter value'
    }
  },
  schema_type: 'Standard',
  created_at: '2025-06-24T00:00:00Z',
  updated_at: '2025-06-24T00:00:00Z'
};

/**
 * Schema with only string fields
 */
export const stringOnlySchema = {
  name: 'text_content',
  state: SCHEMA_STATES.APPROVED,
  fields: {
    title: { 
      field_type: 'String',
      description: 'Content title'
    },
    body: { 
      field_type: 'String',
      description: 'Content body text'
    },
    author: { 
      field_type: 'String',
      description: 'Content author'
    },
    tags: { 
      field_type: 'String',
      description: 'Comma-separated tags'
    }
  },
  schema_type: 'Standard',
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
    fields: {
      [rangeKey]: { field_type: 'Range', description: `Range key: ${rangeKey}` },
      value: { field_type: 'Range', description: 'Range value' }
    },
    schema_type: {
      Range: { range_key: rangeKey }
    },
    rangeInfo: {
      isRangeSchema: true,
      rangeField: {
        name: rangeKey,
        type: 'Range'
      }
    },
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
        fields: {
          range_key: { field_type: 'Range', description: 'Range key field' },
          data: { field_type: 'Range', description: 'Range data field' }
        },
        schema_type: {
          Range: { range_key: 'range_key' }
        },
        rangeInfo: {
          isRangeSchema: true,
          rangeField: {
            name: 'range_key',
            type: 'Range'
          }
        }
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
    typeof schema.fields === 'object' &&
    schema.fields !== null &&
    Object.keys(schema.fields).length > 0
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
  
  const hasRangeType = schema.schema_type?.Range?.range_key;
  const hasRangeFields = Object.values(schema.fields).every(
    field => field.field_type === 'Range'
  );
  const hasRangeInfo = schema.rangeInfo?.isRangeSchema;
  
  return hasRangeType && hasRangeFields && hasRangeInfo;
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