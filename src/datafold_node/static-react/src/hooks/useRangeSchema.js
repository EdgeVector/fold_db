/**
 * Custom hook for range schema operations
 * Centralizes range schema detection and handling logic
 */

import { useCallback } from 'react';
import { 
  RANGE_SCHEMA_CONFIG,
  VALIDATION_MESSAGES,
  FORM_VALIDATION_DEBOUNCE_MS
} from '../constants/schemas.js';

/**
 * Hook for range schema operations and utilities
 * 
 * @returns {Object} Hook result object
 * @returns {Function} isRange - Check if schema is a range schema
 * @returns {Function} min - Get minimum range value (placeholder for future use)
 * @returns {Function} max - Get maximum range value (placeholder for future use)  
 * @returns {Function} step - Get range step value (placeholder for future use)
 * @returns {Object} rangeProps - Collection of range-related functions
 */
export function useRangeSchema() {
  /**
   * Detects if a schema is a range schema
   * Range schemas have:
   * 1. A range_key field defined in the schema
   * 2. All fields have field_type: "Range"
   */
  const isRange = useCallback((schema) => {
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
  }, []);

  /**
   * Gets the range key field name for a range schema
   */
  const getRangeKey = useCallback((schema) => {
    // Check new schema_type structure first, then fall back to old format
    return schema?.schema_type?.Range?.range_key || schema?.range_key || null;
  }, []);

  /**
   * Gets all non-range-key fields for a range schema
   */
  const getNonRangeKeyFields = useCallback((schema) => {
    if (!isRange(schema)) {
      return {};
    }
    
    const rangeKey = getRangeKey(schema);
    const fields = { ...schema.fields };
    
    // Remove the range key field from the list
    if (rangeKey && fields[rangeKey]) {
      delete fields[rangeKey];
    }
    
    return fields;
  }, [isRange, getRangeKey]);

  /**
   * Validates range_key for range schema mutations with debouncing consideration
   */
  const validateRangeKey = useCallback((rangeKeyValue, isRequired = true) => {
    // First check for whitespace-only strings specifically
    if (rangeKeyValue && typeof rangeKeyValue === 'string' && rangeKeyValue.length > 0 && rangeKeyValue.trim().length === 0) {
      return VALIDATION_MESSAGES.RANGE_KEY_EMPTY;
    }
    
    // Then check for required but missing/empty
    if (isRequired && (!rangeKeyValue || !rangeKeyValue.trim())) {
      return VALIDATION_MESSAGES.RANGE_KEY_REQUIRED;
    }
    
    return null;
  }, []);

  /**
   * Enhanced range schema mutation formatter with better validation
   * Range schemas require non-range_key fields to be JSON objects
   */
  const formatRangeMutation = useCallback((schema, mutationType, rangeKeyValue, fieldData) => {
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
          if (typeof fieldValue === 'string' || typeof fieldValue === 'number' || typeof fieldValue === 'boolean') {
            data[fieldName] = { [RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY]: fieldValue };
          } else if (typeof fieldValue === 'object' && fieldValue !== null) {
            // If already an object, use as-is
            data[fieldName] = fieldValue;
          } else {
            // For other types, wrap in an object
            data[fieldName] = { [RANGE_SCHEMA_CONFIG.MUTATION_WRAPPER_KEY]: fieldValue };
          }
        }
      });
      
      mutation.data = data;
    }
    
    return mutation;
  }, [getRangeKey]);

  /**
   * Formats a range schema query with proper range_filter
   */
  const formatRangeQuery = useCallback((schema, fields, rangeFilterValue) => {
    const query = {
      type: 'query',
      schema: schema.name,
      fields: fields
    };
    
    if (rangeFilterValue && rangeFilterValue.trim()) {
      query.range_filter = { Key: rangeFilterValue.trim() };
    }
    
    return query;
  }, []);

  /**
   * Gets range schema display information
   */
  const getRangeSchemaInfo = useCallback((schema) => {
    if (!isRange(schema)) {
      return null;
    }
    
    return {
      isRangeSchema: true,
      rangeKey: getRangeKey(schema),
      rangeFields: Object.entries(schema.fields || {}).filter(([_, field]) => field.field_type === RANGE_SCHEMA_CONFIG.FIELD_TYPE),
      nonRangeKeyFields: getNonRangeKeyFields(schema),
      totalFields: Object.keys(schema.fields || {}).length
    };
  }, [isRange, getRangeKey, getNonRangeKeyFields]);

  /**
   * Gets range fields from a schema
   */
  const getRangeFields = useCallback((schema) => {
    if (!schema || !schema.fields) return [];
    
    return Object.entries(schema.fields)
      .filter(([_, field]) => field.field_type === RANGE_SCHEMA_CONFIG.FIELD_TYPE)
      .map(([fieldName]) => fieldName);
  }, []);

  /**
   * Gets non-range fields from a schema
   */
  const getNonRangeFields = useCallback((schema) => {
    if (!schema || !schema.fields) return {};
    
    return Object.fromEntries(
      Object.entries(schema.fields).filter(
        ([_, field]) => field.field_type !== RANGE_SCHEMA_CONFIG.FIELD_TYPE
      )
    );
  }, []);

  // Placeholder functions for future range constraints
  const min = useCallback(() => null, []);
  const max = useCallback(() => null, []);
  const step = useCallback(() => null, []);

  // Collection of all range-related properties and functions
  const rangeProps = {
    isRange,
    getRangeKey,
    getNonRangeKeyFields,
    validateRangeKey,
    formatRangeMutation,
    formatRangeQuery,
    getRangeSchemaInfo,
    getRangeFields,
    getNonRangeFields,
    debounceMs: FORM_VALIDATION_DEBOUNCE_MS
  };

  return {
    isRange,
    min,
    max,
    step,
    rangeProps
  };
}

export default useRangeSchema;