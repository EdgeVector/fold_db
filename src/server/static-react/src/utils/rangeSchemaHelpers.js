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

import { RANGE_SCHEMA_CONFIG } from '../constants/schemas.js';
import { VALIDATION_MESSAGES } from '../constants/validation.js';
import { MUTATION_TYPE_API_MAP } from '../constants/ui.js';

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
 * Gets the schema type from a schema object.
 * Handles the simple string format from Rust: "Single" | "Range" | "HashRange"
 * 
 * @param {Schema} schema - Schema object
 * @returns {'Single'|'Range'|'HashRange'|null} Schema type or null if not determinable
 */
export function getSchemaType(schema) {
  if (!schema || typeof schema !== 'object') return null;
  
  const schemaType = schema.schema_type;
  
  // Handle string types (simplified format)
  if (schemaType === 'Single') {
    return 'Single';
  }
  if (schemaType === 'Range') {
    return 'Range';
  }
  if (schemaType === 'HashRange') {
    return 'HashRange';
  }
  
  // Legacy: Handle old object types for backward compatibility
  if (typeof schemaType === 'object' && schemaType !== null) {
    if ('HashRange' in schemaType) {
      return 'HashRange';
    }
    if ('Range' in schemaType) {
      return 'Range';
    }
  }
  
  return null;
}

/**
 * Detects if a schema is a HashRange schema
 * HashRange schemas have schema_type: "HashRange"
 * Backend is authoritative - no front-end validation
 * 
 * @param {Schema} schema - Schema object to check
 * @returns {boolean} True if schema is a HashRange schema
 */
export function isHashRangeSchema(schema) {
  if (!schema || typeof schema !== 'object') {
    return false;
  }
  
  // Trust backend schema_type - it knows best
  return getSchemaType(schema) === 'HashRange';
}

/**
 * Gets the hash field expression for a HashRange schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Hash field expression or null if not found
 */
export function getHashField(schema) {
  if (!schema || !isHashRangeSchema(schema)) return null;
  return schema.key?.hash_field || null;
}

/**
 * Gets the range field expression for a HashRange schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Range field expression or null if not found
 */
export function getRangeField(schema) {
  if (!schema || !isHashRangeSchema(schema)) return null;
  return schema.key?.range_field || null;
}

/**
 * Returns the last segment of a dotted field expression.
 * @param {string} expr
 * @returns {string|null}
 */
function lastSegment(expr) {
  if (typeof expr !== 'string') return null;
  const parts = expr.split('.');
  return parts[parts.length - 1] || expr;
}

/**
 * Detects if a schema is a range schema
 * Range schemas have schema_type: "Range"
 * Backend is authoritative - no front-end validation
 * 
 * @param {Schema} schema - Schema object to check
 * @returns {boolean} True if schema is a range schema
 */
export function isRangeSchema(schema) {
  if (!schema || typeof schema !== 'object') {
    return false;
  }
  
  // Trust backend schema_type - it knows best
  return getSchemaType(schema) === 'Range';
}

/**
 * Gets the range key field name for a range schema
 * 
 * @param {Schema} schema - Schema object
 * @returns {string|null} Range key field name or null if not found
 */
export function getRangeKey(schema) {
  if (!schema || typeof schema !== 'object') return null;
  
  // Use the unified key field for all schema types
  const rf = schema?.key?.range_field;
  return (typeof rf === 'string' && rf.trim()) ? lastSegment(rf) : null;
}

/**
 * Gets the hash key field name for any schema type (Single, Range, HashRange).
 * Returns the last segment of the expression if dotted, else the raw field.
 * @param {Schema} schema
 * @returns {string|null}
 */
export function getHashKey(schema) {
  if (!schema || typeof schema !== 'object') return null;
  const hf = schema?.key?.hash_field;
  return hf && typeof hf === 'string' && hf.trim() ? lastSegment(hf) : null;
}

/**
 * Returns a normalized key shape for the schema.
 * { type: 'single'|'range'|'hashrange', hashField: string|null, rangeField: string|null }
 * @param {Schema} schema
 * @returns {{type:string, hashField:string|null, rangeField:string|null}}
 */
export function getKeyShape(schema) {
  const hashField = getHashKey(schema);
  const rangeField = getRangeKey(schema);
  let type = 'single';
  if (isHashRangeSchema(schema)) type = 'hashrange';
  else if (isRangeSchema(schema)) type = 'range';
  return { type, hashField, rangeField };
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
  
  // Declarative schema fields are an array of strings - convert to object format
  if (!Array.isArray(schema.fields)) {
    throw new Error(`Expected schema.fields to be an array for range schema "${schema.name}", got ${typeof schema.fields}`);
  }
  
  return schema.fields.reduce((acc, fieldName) => {
    if (fieldName !== rangeKey) {
      acc[fieldName] = {};
    }
    return acc;
  }, {});
}

/**
 * Gets range fields from a schema
 * Note: Declarative schemas don't store field types, so this returns empty array
 * 
 * @param {Schema} schema - Schema object
 * @returns {string[]} Array of range field names (empty for declarative schemas)
 */
export function getRangeFields(schema) {
  if (!schema || !schema.fields) return [];
  // Declarative schemas don't have field_type metadata
  return [];
}

/**
 * Gets non-range fields from a schema
 * Note: Declarative schemas don't have field_type metadata
 * 
 * @param {Schema} schema - Schema object
 * @returns {Object} Object containing all fields (declarative schemas don't distinguish by type)
 */
export function getNonRangeFields(schema) {
  if (!schema) {
    throw new Error('Schema is required for getNonRangeFields');
  }
  if (!schema.fields) {
    throw new Error(`Schema "${schema.name}" is missing fields property`);
  }
  if (!Array.isArray(schema.fields)) {
    throw new Error(`Expected schema.fields to be an array for schema "${schema.name}", got ${typeof schema.fields}`);
  }
  
  return schema.fields.reduce((acc, fieldName) => {
    acc[fieldName] = {};
    return acc;
  }, {});
}

/**
 * Validates range_key for range schema mutations
 * Minimal validation - backend is authoritative
 * 
 * @param {string} rangeKeyValue - Range key value to validate
 * @param {boolean} [isRequired=true] - Whether range key is required
 * @returns {string|null} Error message or null if valid
 */
export function validateRangeKey(rangeKeyValue, isRequired = true) {
  // Only check if required field is completely missing - backend handles everything else
  if (isRequired && !rangeKeyValue) {
    return 'Range key is required';
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
  const normalizedMutationType = typeof mutationType === 'string'
    ? (MUTATION_TYPE_API_MAP[mutationType] || mutationType.toLowerCase())
    : '';
  const isDeleteOperation = normalizedMutationType === 'delete';
  const mutation = {
    type: 'mutation',
    schema: schema.name,
    mutation_type: normalizedMutationType
  };
  
  // Get the actual range key field name from the schema
  const rangeKeyFieldName = getRangeKey(schema);
  
  if (isDeleteOperation) {
    mutation.fields_and_values = {};
    mutation.key_value = { hash: null, range: null };
    // For delete operations, use the actual range key field name
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      mutation.fields_and_values[rangeKeyFieldName] = rangeKeyValue.trim();
      mutation.key_value.range = rangeKeyValue.trim();
    }
  } else {
    const fieldsAndValues = {};
    
    // Add range key using the actual field name from schema (as primitive value)
    if (rangeKeyValue && rangeKeyValue.trim() && rangeKeyFieldName) {
      fieldsAndValues[rangeKeyFieldName] = rangeKeyValue.trim();
    }
    
    // Format non-range_key fields as JSON objects for range schemas
    // The backend expects non-range_key fields to be objects so it can inject the range_key
    Object.entries(fieldData).forEach(([fieldName, fieldValue]) => {
      if (fieldName !== rangeKeyFieldName) {
        // Convert simple values to JSON objects with a 'value' key
        const wrapperKey = RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY || 'value';
        
        if (typeof fieldValue === 'string' || typeof fieldValue === 'number' || typeof fieldValue === 'boolean') {
          fieldsAndValues[fieldName] = { [wrapperKey]: fieldValue };
        } else if (typeof fieldValue === 'object' && fieldValue !== null) {
          // If already an object, use as-is
          fieldsAndValues[fieldName] = fieldValue;
        } else {
          // For other types, wrap in an object
          fieldsAndValues[fieldName] = { [wrapperKey]: fieldValue };
        }
      }
    });
    
    mutation.fields_and_values = fieldsAndValues;
    mutation.key_value = { 
      hash: null, 
      range: rangeKeyValue && rangeKeyValue.trim() ? rangeKeyValue.trim() : null 
    };
  }
  
  return mutation;
}

/**
 * Formats a range schema query with proper HashRangeFilter
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
    // Use RangeKey filter for exact range key match
    query.filter = { "RangeKey": rangeFilterValue.trim() };
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
  
  if (hashKey && hashKey.trim()) {
    query.filter = { "HashKey": hashKey.trim() };
  } else if (rangeKey && rangeKey.trim()) {
    query.filter = { "RangeKey": rangeKey.trim() };
  }
  
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
    rangeFields: [],  // Declarative schemas don't store field types
    nonRangeKeyFields: getNonRangeKeyFields(schema),
    totalFields: Array.isArray(schema.fields) ? schema.fields.length : 0
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
 * Checks if a value is considered empty
 * Minimal check - backend handles detailed validation
 * 
 * @param {*} value - Value to check
 * @returns {boolean} True if value is empty
 */
export function isValueEmpty(value) {
  // Only check for null/undefined - backend validates everything else
  return value === null || value === undefined;
}


// From useRangeSchema.js hook - alias exports
export const isRange = isRangeSchema;

/**
 * Gets HashRange schema display information
 * @param {Schema} schema - Schema object to analyze
 * @returns {Object|null} HashRange schema info or null if not HashRange
 */
export function getHashRangeSchemaInfo(schema) {
  if (!isHashRangeSchema(schema)) {
    return null;
  }
  
  return {
    isHashRangeSchema: true,
    hashField: getHashKey(schema),
    rangeField: getRangeKey(schema),
    totalFields: Array.isArray(schema.fields) ? schema.fields.length : 0
  };
}
