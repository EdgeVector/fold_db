/**
 * useQueryBuilder Hook
 * Handles query building logic with validation
 * Part of UCR-1-5: Create QueryBuilder hook for complex query construction
 */

import { useMemo, useCallback } from 'react';
import { useAppSelector } from '../store/hooks';
import { selectApprovedSchemas } from '../store/schemaSlice';
import { isHashRangeSchema, formatHashRangeQuery } from '../utils/rangeSchemaHelpers.js';

/**
 * Query builder hook that handles query construction and validation
 * 
 * @param {Object} options - Configuration options
 * @param {string} options.schema - Selected schema name
 * @param {Object} options.queryState - Current query state
 * @param {Array} options.queryState.queryFields - Array of selected field names
 * @param {Object} options.queryState.fieldValues - Object with field values
 * @param {Object} options.queryState.rangeFilters - Range filter configurations
 * @param {Array} options.queryState.filters - Query filters
 * @param {Object} options.queryState.orderBy - Order by configuration
 * @param {Object} options.schemas - Available schemas
 * @returns {Object} Query builder state and methods
 */
export function useQueryBuilder({ schema, queryState, schemas }) {
  const approvedSchemas = useAppSelector(selectApprovedSchemas);
  
  // Get the selected schema object
  const selectedSchemaObj = useMemo(() => {
    if (schemas && schemas[schema]) {
      return schemas[schema];
    }
    // approvedSchemas is now an array, not an object
    if (approvedSchemas && Array.isArray(approvedSchemas)) {
      return approvedSchemas.find(s => s.name === schema) || null;
    }
    return null;
  }, [schema, schemas, approvedSchemas]);

  // Validation logic
  const validationErrors = useMemo(() => {
    const errors = [];
    
    // Basic schema validation
    if (!schema) {
      errors.push('Schema selection is required');
      return errors;
    }

    if (!selectedSchemaObj) {
      errors.push('Selected schema not found');
      return errors;
    }

    if (!queryState) {
      return errors;
    }

    const { queryFields = [], fieldValues = {}, rangeFilters = {}, filters = [] } = queryState;

    // If no fields are selected, only validate basic schema requirements
    if (queryFields.length === 0) {
      // For schemas without fields, this is valid
      if (!selectedSchemaObj.fields || Object.keys(selectedSchemaObj.fields).length === 0) {
        return errors;
      }
      // For range schemas with no fields selected, this is also valid (no range key required)
      if (selectedSchemaObj.schema_type === 'Range') {
        return errors;
      }
      // Otherwise require at least one field
      errors.push('At least one field must be selected');
      return errors;
    }

    // Validate required fields that are selected
    if (selectedSchemaObj.fields) {
      queryFields.forEach(fieldName => {
        const fieldDef = selectedSchemaObj.fields[fieldName];
        const value = fieldValues[fieldName];
        
        if (fieldDef) {
          // Check if required field is missing or empty
          if (fieldDef.required) {
            if (!(fieldName in fieldValues)) {
              errors.push(`Required field "${fieldName}" is missing`);
            } else if (value === null || value === '') {
              errors.push(`Required field "${fieldName}" cannot be empty`);
            }
          }
          
          // Validate field types for non-empty values
          if (value && value !== '') {
            if (fieldDef.field_type === 'Integer' || fieldDef.field_type === 'Number') {
              if (isNaN(Number(value))) {
                errors.push(`Field "${fieldName}" must be a number`);
              }
            }
          }
        }
      });
    }

    // Validate range schema requirements
    if (selectedSchemaObj.schema_type === 'Range' && queryFields.length > 0) {
      const hasRangeKey = rangeFilters && Object.keys(rangeFilters).some(key =>
        rangeFilters[key]?.key
      );
      if (!hasRangeKey) {
        errors.push('Range key missing for range schema');
      }
    }

    // Validate filters against schema (only if filters exist)
    if (filters && Array.isArray(filters) && filters.length > 0) {
      filters.forEach(filter => {
        if (selectedSchemaObj.fields && !selectedSchemaObj.fields[filter.field]) {
          errors.push(`Filter field "${filter.field}" does not exist in schema`);
        }
      });
    }

    return errors;
  }, [schema, selectedSchemaObj, queryState]);

  const isValid = validationErrors.length === 0;

  // Build query object
  const query = useMemo(() => {
    if (!schema || !queryState || !selectedSchemaObj) {
      return {};
    }

    const { queryFields = [], fieldValues = {}, rangeFilters = {}, filters = [], orderBy } = queryState;
    
    // Build query with selected fields and their values
    const builtQuery = {
      type: "query", // Required field for server parsing
      schema,
      fields: queryFields, // Array of selected field names as expected by server
      queryFields // Also include queryFields for compatibility
    };

    // Add field values if there are any (for range keys or other purposes)
    if (fieldValues && Object.keys(fieldValues).length > 0) {
      builtQuery.fieldValues = fieldValues;
    }

    // Handle HashRange schema queries
    if (isHashRangeSchema(selectedSchemaObj)) {
      const hashKey = queryState.hashKeyValue;
      const rangeKey = queryState.rangeSchemaFilter?.key;
      
      if (hashKey || rangeKey) {
        builtQuery.filter = {};
        
        if (hashKey && hashKey.trim()) {
          builtQuery.filter.hash_filter = {
            Key: hashKey.trim()
          };
        }
        
        if (rangeKey && rangeKey.trim()) {
          // For HashRange schemas, range key filtering would go here if needed
          // Currently the backend only supports hash key filtering
        }
      }
    }

    // Add range schema filter for range schemas (this is the correct one for Range schemas)
    if (selectedSchemaObj.schema_type?.Range?.range_key && queryState.rangeSchemaFilter) {
      const rangeSchemaFilter = queryState.rangeSchemaFilter;
      const rangeKey = selectedSchemaObj.schema_type.Range.range_key;
      
      if (rangeKey) {
        // Determine which filter type to use based on what's filled in
        let filterType = null;
        let filterValue = null;
        
        if (rangeSchemaFilter.key) {
          filterType = 'Key';
          filterValue = rangeSchemaFilter.key;
        } else if (rangeSchemaFilter.keyPrefix) {
          filterType = 'KeyPrefix';
          filterValue = rangeSchemaFilter.keyPrefix;
        } else if (rangeSchemaFilter.start && rangeSchemaFilter.end) {
          filterType = 'KeyRange';
          filterValue = { start: rangeSchemaFilter.start, end: rangeSchemaFilter.end };
        }
        
        if (filterType && filterValue) {
          builtQuery.filter = {
            range_filter: {
              [rangeKey]: {
                [filterType]: filterValue
              }
            }
          };
        }
      }
    }

    // Add filters
    if (filters && filters.length > 0) {
      builtQuery.filters = filters;
    }

    // Add orderBy
    if (orderBy && orderBy.field) {
      builtQuery.orderBy = orderBy;
    }

    return builtQuery;
  }, [schema, queryState, selectedSchemaObj]);

  // Manual build function
  const buildQuery = useCallback(() => {
    return query;
  }, [query]);

  // Manual validation function
  const validateQuery = useCallback(() => {
    return {
      isValid,
      errors: validationErrors
    };
  }, [isValid, validationErrors]);

  return {
    query,
    validationErrors,
    isValid,
    buildQuery,
    validateQuery
  };
}

export default useQueryBuilder;