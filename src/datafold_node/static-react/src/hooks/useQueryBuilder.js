/**
 * useQueryBuilder Hook
 * Handles query building logic with validation
 * Part of UCR-1-5: Create QueryBuilder hook for complex query construction
 */

import { useMemo, useCallback } from 'react';
import { useAppSelector } from '../store/hooks';

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
  const approvedSchemas = useAppSelector(state => state.schema.approved || {});
  
  // Get the selected schema object
  const selectedSchemaObj = useMemo(() => {
    if (schemas && schemas[schema]) {
      return schemas[schema];
    }
    if (approvedSchemas && approvedSchemas[schema]) {
      return approvedSchemas[schema];
    }
    return null;
  }, [schema, schemas, approvedSchemas]);

  // Validation logic
  const validationErrors = useMemo(() => {
    const errors = [];
    
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

    // Check required fields
    if (selectedSchemaObj.fields) {
      Object.entries(selectedSchemaObj.fields).forEach(([fieldName, fieldDef]) => {
        if (fieldDef.required && queryFields.includes(fieldName)) {
          const value = fieldValues[fieldName];
          if (value === undefined || value === null || value === '') {
            errors.push(`Required field "${fieldName}" is missing`);
          }
        }
      });
    }

    // Check range schema requirements
    if (selectedSchemaObj.schema_type === 'Range' && queryFields.length > 0) {
      const hasRangeKey = rangeFilters && Object.keys(rangeFilters).some(key => rangeFilters[key]?.key);
      if (!hasRangeKey) {
        errors.push('Range key missing for range schema');
      }
    }

    // Validate field types
    if (selectedSchemaObj.fields) {
      queryFields.forEach(fieldName => {
        const fieldDef = selectedSchemaObj.fields[fieldName];
        const value = fieldValues[fieldName];
        
        if (fieldDef && value !== undefined && value !== null && value !== '') {
          if (fieldDef.field_type === 'Integer' || fieldDef.field_type === 'Number') {
            if (isNaN(Number(value))) {
              errors.push(`Field "${fieldName}" must be a number`);
            }
          }
        }
      });
    }

    // Validate filters against schema
    filters.forEach(filter => {
      if (selectedSchemaObj.fields && !selectedSchemaObj.fields[filter.field]) {
        errors.push(`Filter field "${filter.field}" does not exist in schema`);
      }
    });

    // Validate empty field values
    queryFields.forEach(fieldName => {
      const fieldDef = selectedSchemaObj.fields?.[fieldName];
      const value = fieldValues[fieldName];
      
      if (fieldDef?.required && (value === '' || value === null || value === undefined)) {
        errors.push(`Required field "${fieldName}" cannot be empty`);
      }
    });

    return errors;
  }, [schema, selectedSchemaObj, queryState]);

  const isValid = validationErrors.length === 0;

  // Build query object
  const query = useMemo(() => {
    if (!schema || !queryState || !selectedSchemaObj) {
      return {};
    }

    const { queryFields = [], fieldValues = {}, rangeFilters = {}, filters = [], orderBy } = queryState;
    
    const builtQuery = {
      schema,
      fields: fieldValues
    };

    // Add range key for range schemas
    if (selectedSchemaObj.schema_type === 'Range' && rangeFilters) {
      const rangeField = Object.keys(rangeFilters).find(key => rangeFilters[key]?.key);
      if (rangeField && rangeFilters[rangeField]?.key) {
        builtQuery.rangeKey = rangeFilters[rangeField].key;
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