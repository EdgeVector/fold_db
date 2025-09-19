/**
 * Range Schema Utilities - Consolidated Implementation
 * TASK-008: Duplicate Code Detection and Elimination
 * 
 * This module consolidates range schema utilities that were duplicated across
 * useRangeSchema.js and rangeSchemaUtils.js, providing a single source of truth
 * for range schema detection, validation, and formatting operations.
 * 
 * Range schemas are specialized schemas designed for time-series and ordered data
 * with the following characteristics:
 * - Contains a designated range_key field for ordering
 * - All fields have field_type: "Range" 
 * - Non-range_key fields are wrapped in objects for backend processing
 * - Supports efficient range-based queries and mutations
 */

import { 
  RANGE_SCHEMA_CONFIG, 
  VALIDATION_MESSAGES 
} from '../constants/schemas.js';

/**
 * @typedef {Object} Schema
 * @property {string} name - Schema name
 * @property {Object} fields - Field definitions
 * @property {Object} [schema_type] - Schema type information
 * @property {string} [range_key] - Legacy range key field name
 */

/**
 * @typedef {Object} RangeSchemaInfo
 * @property {boolean} isRangeSchema - Whether this is a range schema
 * @property {string|null} rangeKey - Name of the range key field
 * @property {Array<[string, Object]>} rangeFields - Array of [fieldName, fieldDef] for range fields
 * @property {Object} nonRangeKeyFields - Object containing non-range-key fields
 * @property {number} totalFields - Total number of fields in schema
 */

/**
 * Detects if a schema is a HashRange schema
 * HashRange schemas have:
 * 1. schema_type: "HashRange"
 * 2. Fields with field_type: "HashRange" that contain hash_field and range_field
 * 
 * @param {Schema} schema - Schema object to check
 * @returns {boolean} True if schema is a HashRange schema
 */
export function isHashRangeSchema(schema) {
  if (!schema || typeof schema !== 'object') {
    return false;
  }
  
  // Prefer universal key detection for HashRange
  if (schema.schema_type === 'HashRange' && schema.key && typeof schema.key === 'object') {
    const { hash_field, range_field } = schema.key || {};
    return typeof hash_field === 'string' && hash_field.trim() && typeof range_field === 'string' && range_field.trim();
  }
  
  // Fallback: legacy field-based detection
  return schema.schema_type === 'HashRange' &&
    schema.fields &&
    typeof schema.fields === 'object' &&
    Object.values(schema.fields).some(field =>
      field.field_type === 'HashRange' &&
      field.hash_field &&
      field.range_field
    );
}

/**
 * Gets the hash field expression for a HashRange schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Hash field expression or null if not found
 */
export function getHashField(schema) {
  if (!schema || schema.schema_type !== 'HashRange') return null;
  // Prefer universal key
  if (schema.key && typeof schema.key?.hash_field === 'string' && schema.key.hash_field.trim()) {
    return schema.key.hash_field;
  }
  // Fallback to first HashRange field definition
  const hashRangeField = Object.values(schema.fields || {}).find(field => field.field_type === 'HashRange' && field.hash_field);
  return hashRangeField ? hashRangeField.hash_field : null;
}

/**
 * Gets the range field expression for a HashRange schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Range field expression or null if not found
 */
export function getRangeField(schema) {
  if (!schema || schema.schema_type !== 'HashRange') return null;
  // Prefer universal key
  if (schema.key && typeof schema.key?.range_field === 'string' && schema.key.range_field.trim()) {
    return schema.key.range_field;
  }
  // Fallback to first HashRange field definition
  const hashRangeField = Object.values(schema.fields || {}).find(field => field.field_type === 'HashRange' && field.range_field);
  return hashRangeField ? hashRangeField.range_field : null;
}

/**
 * Detects if a schema is a range schema
 * Range schemas have:
 * 1. A range_key field defined in the schema
 * 2. All fields have field_type: "Range"
 * 
 * @param {Schema} schema - Schema object to check
 * @returns {boolean} True if schema is a range schema
 */
export function isRangeSchema(schema) {
  // Enhanced range schema detection with better validation
  if (!schema || typeof schema !== 'object') {
    return false;
  }
  
  // Check for range_key in the new schema_type structure or old format
  const hasRangeKey = schema.schema_type?.Range?.range_key || schema.range_key;
  if (!hasRangeKey || typeof hasRangeKey !== 'string') {
    return false;
  }
  
  if (!schema.fields || typeof schema.fields !== 'object') {
    return false;
  }
  
  // Check if all fields have field_type: "Range"
  const fieldEntries = Object.entries(schema.fields);
  if (fieldEntries.length === 0) {
    return false;
  }
  
  // More robust field type checking
  const allFieldsAreRange = fieldEntries.every(([fieldName, field]) => {
    if (!field || typeof field !== 'object') {
      console.warn(`Field ${fieldName} is not a valid field object in schema ${schema.name}`);
      return false;
    }
    
    if (field.field_type !== RANGE_SCHEMA_CONFIG.FIELD_TYPE) {
      console.warn(`Field ${fieldName} has field_type "${field.field_type}", expected "${RANGE_SCHEMA_CONFIG.FIELD_TYPE}" in schema ${schema.name}`);
      return false;
    }
    
    return true;
  });
  
  return allFieldsAreRange;
}

/**
 * Gets the range key field name for a range schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Range key field name or null if not found
 */
export function getRangeKey(schema) {
  // Prefer universal key on Range if present
  if (schema?.schema_type?.Range) {
    const universalRange = schema?.key?.range_field;
    if (typeof universalRange === 'string' && universalRange.trim()) {
      // Show last segment for readability
      const parts = universalRange.split('.');
      return parts[parts.length - 1] || universalRange;
    }
    // Fallback to legacy
    return schema?.schema_type?.Range?.range_key || schema?.range_key || null;
  }
  return schema?.range_key || null;
}

/**
 * Gets all non-range-key fields for a range schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {Object} Object containing non-range-key fields
 */
export function getNonRangeKeyFields(schema) {
  if (!isRangeSchema(schema)) {
    return {};
  }
  
  const rangeKey = getRangeKey(schema);
  const fields = { ...schema.fields };
  
  // Remove the range key field from the list
  if (rangeKey && fields[rangeKey]) {
    delete fields[rangeKey];
  }
  
  return fields;
}

/**
 * Gets range fields from a schema (fields with field_type: "Range")
 * 
 * @param {Schema} schema - Schema object
 * @returns {string[]} Array of range field names
 */
export function getRangeFields(schema) {
  if (!schema || !schema.fields) return [];
  
  return Object.entries(schema.fields)
    .filter(([_, field]) => field.field_type === RANGE_SCHEMA_CONFIG.FIELD_TYPE)
    .map(([fieldName]) => fieldName);
}

/**
 * Gets non-range fields from a schema (fields without field_type: "Range")
 * 
 * @param {Schema} schema - Schema object
 * @returns {Object} Object containing non-range fields
 */
export function getNonRangeFields(schema) {
  if (!schema || !schema.fields) return {};
  
  return Object.fromEntries(
    Object.entries(schema.fields).filter(
      ([_, field]) => field.field_type !== RANGE_SCHEMA_CONFIG.FIELD_TYPE
    )
  );
}

/**
 * Validates range_key for range schema mutations
 * 
 * @param {string} rangeKeyValue - Range key value to validate
 * @param {boolean} [isRequired=true] - Whether range key is required
 * @returns {string|null} Error message or null if valid
 */
export function validateRangeKey(rangeKeyValue, isRequired = true) {
  // First check for whitespace-only strings specifically
  if (rangeKeyValue && typeof rangeKeyValue === 'string' && rangeKeyValue.length > 0 && rangeKeyValue.trim().length === 0) {
    return VALIDATION_MESSAGES.RANGE_KEY_EMPTY || 'Range key cannot be empty';
  }
  
  // Then check for required but missing/empty
  if (isRequired && (!rangeKeyValue || !rangeKeyValue.trim())) {
    return VALIDATION_MESSAGES.RANGE_KEY_REQUIRED || 'Range key is required for range schema mutations';
  }
  
  return null;
}

/**
 * Enhanced range schema mutation formatter with better validation
 * Range schemas require non-range_key fields to be JSON objects
 * 
 * @param {Schema} schema - Schema object
 * @param {string} mutationType - Mutation type (Create, Update, Delete)
 * @param {string} rangeKeyValue - Range key value
 * @param {Object} fieldData - Field data for mutation
 * @returns {Object} Formatted mutation object
 */
export function formatRangeMutation(schema, mutationType, rangeKeyValue, fieldData) {
  const mutation = {
    type: 'mutation',
    schema: schema.name,
    mutation_type: mutationType.toLowerCase()
  };
  
  // Get the actual range key field name from the schema
  const rangeKeyFieldName = getRangeKey(schema);
  
  if (mutationType === 'Delete') {
    mutation.data = {};
    // For delete operations, use the actual range key field name
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      mutation.data[rangeKeyFieldName] = rangeKeyValue.trim();
    }
  } else {
    const data = {};
    
    // Add range key using the actual field name from schema (as primitive value)
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      data[rangeKeyFieldName] = rangeKeyValue.trim();
    }
    
    // Format non-range_key fields as JSON objects for range schemas
    // The backend expects non-range_key fields to be objects so it can inject the range_key
    Object.entries(fieldData).forEach(([fieldName, fieldValue]) => {
      if (fieldName !== rangeKeyFieldName) {
        // Convert simple values to JSON objects with a 'value' key
        const wrapperKey = RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY || 'value';
        
        if (typeof fieldValue === 'string' || typeof fieldValue === 'number' || typeof fieldValue === 'boolean') {
          data[fieldName] = { [wrapperKey]: fieldValue };
        } else if (typeof fieldValue === 'object' && fieldValue !== null) {
          // If already an object, use as-is
          data[fieldName] = fieldValue;
        } else {
          // For other types, wrap in an object
          data[fieldName] = { [wrapperKey]: fieldValue };
        }
      }
    });
    
    mutation.data = data;
  }
  
  return mutation;
}

/**
 * Formats a range schema query with proper range_filter
 * 
 * @param {Schema} schema - Schema object
 * @param {string[]} fields - Fields to query
 * @param {string} [rangeFilterValue] - Range filter value
 * @returns {Object} Formatted query object
 */
export function formatRangeQuery(schema, fields, rangeFilterValue) {
  const query = {
    type: 'query',
    schema: schema.name,
    fields: fields
  };
  
  if (rangeFilterValue && rangeFilterValue.trim()) {
    query.range_filter = { Key: rangeFilterValue.trim() };
  }
  
  return query;
}

/**
 * Formats a HashRange schema query with hash and range key filters
 * 
 * @param {Schema} schema - Schema object
 * @param {string[]} fields - Fields to query
 * @param {string} [hashKey] - Hash key value
 * @param {string} [rangeKey] - Range key value
 * @returns {Object} Formatted query object
 */
export function formatHashRangeQuery(schema, fields, hashKey, rangeKey) {
  const query = {
    type: 'query',
    schema: schema.name,
    fields: fields
  };
  
  // Add hash filter if hash key is provided
  if (hashKey && hashKey.trim()) {
    query.filter = {
      hash_filter: {
        Key: hashKey.trim()
      }
    };
  }
  
  // Note: Range key filtering for HashRange schemas is not currently supported
  // by the backend, so rangeKey parameter is ignored
  
  return query;
}

/**
 * Gets comprehensive range schema display information
 * 
 * @param {Schema} schema - Schema object
 * @returns {RangeSchemaInfo|null} Range schema info or null if not a range schema
 */
export function getRangeSchemaInfo(schema) {
  if (!isRangeSchema(schema)) {
    return null;
  }
  
  return {
    isRangeSchema: true,
    rangeKey: getRangeKey(schema),
    rangeFields: Object.entries(schema.fields || {}).filter(([_, field]) => field.field_type === RANGE_SCHEMA_CONFIG.FIELD_TYPE),
    nonRangeKeyFields: getNonRangeKeyFields(schema),
    totalFields: Object.keys(schema.fields || {}).length
  };
}

/**
 * Normalizes schema state to lowercase string for consistent comparison
 * This addresses duplication in schema state checking across multiple files
 * 
 * @param {*} state - Schema state in various formats
 * @returns {string} Normalized state string
 */
export function normalizeSchemaState(state) {
  if (typeof state === 'string') return state.toLowerCase();
  if (typeof state === 'object' && state !== null) {
    // Handle object format like { state: 'approved' }
    if (state.state) {
      return String(state.state).toLowerCase();
    }
    return String(state).toLowerCase();
  }
  return String(state || '').toLowerCase();
}

/**
 * Checks if a value is considered empty for validation purposes
 * This addresses duplication in empty value checking across validation functions
 * 
 * @param {*} value - Value to check
 * @returns {boolean} True if value is empty
 */
export function isValueEmpty(value) {
  if (value === null || value === undefined) return true;
  if (typeof value === 'string') return value.trim().length === 0;
  if (Array.isArray(value)) return value.length === 0;
  if (typeof value === 'object') return Object.keys(value).length === 0;
  return false;
}

/**
 * Legacy function names for backward compatibility
 * These maintain the original function names from the separate files
 */

// From rangeSchemaUtils.js
export const formatEnhancedRangeSchemaMutation = formatRangeMutation;
export const validateRangeKeyForMutation = validateRangeKey;
export const formatRangeSchemaQuery = formatRangeQuery;

// From useRangeSchema.js hook - alias exports
export const isRange = isRangeSchema;
// Note: formatRangeQuery and formatRangeMutation already exported above as formatRangeSchemaQuery and formatEnhancedRangeSchemaMutation