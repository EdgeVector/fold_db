/**
 * useQueryBuilder Hook
 * Handles query building logic with validation
 * Part of UCR-1-5: Create QueryBuilder hook for complex query construction
 */

import { useMemo, useCallback } from 'react';
import { useAppSelector } from '../store/hooks';
import { selectApprovedSchemas } from '../store/schemaSlice';
import { isHashRangeSchema, isRangeSchema as detectRangeSchema } from '../utils/rangeSchemaHelpers.js';

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
export function useQueryBuilder({
  schema,
  queryState,
  schemas,
  selectedSchemaObj: providedSelectedSchema,
  isRangeSchema: providedIsRangeSchema,
  rangeKey: providedRangeKey
}) {
  const approvedSchemas = useAppSelector(selectApprovedSchemas);

  // Get the selected schema object
  const selectedSchemaObj = useMemo(() => {
    if (providedSelectedSchema) {
      return providedSelectedSchema;
    }

    if (schemas && schema && schemas[schema]) {
      return schemas[schema];
    }
    // approvedSchemas is now an array, not an object
    if (approvedSchemas && Array.isArray(approvedSchemas)) {
      return approvedSchemas.find(s => s.name === schema) || null;
    }
    return null;
  }, [providedSelectedSchema, schema, schemas, approvedSchemas]);

  const schemaIsRange = useMemo(() => {
    if (typeof providedIsRangeSchema === 'boolean') {
      return providedIsRangeSchema;
    }

    if (!selectedSchemaObj) {
      return false;
    }

    // Check for Range schema_type (tagged union format: { "Range": { keyconfig: {...} } })
    const isRangeType = typeof selectedSchemaObj.schema_type === 'object' && 
                       selectedSchemaObj.schema_type !== null &&
                       'Range' in selectedSchemaObj.schema_type;
    
    if (isRangeType) {
      return true;
    }

    if (detectRangeSchema(selectedSchemaObj)) {
      return true;
    }

    if (selectedSchemaObj.fields && typeof selectedSchemaObj.fields === 'object') {
      return Object.values(selectedSchemaObj.fields).some(field => field?.field_type === 'Range');
    }

    return false;
  }, [selectedSchemaObj, providedIsRangeSchema]);

  // Minimal validation - only check basic schema selection
  const validationErrors = useMemo(() => {
    const errors = [];
    
    // Only validate that a schema is selected
    if (!schema) {
      errors.push('Schema selection is required');
      return errors;
    }

    if (!selectedSchemaObj) {
      errors.push('Selected schema not found');
      return errors;
    }

    // All other validation removed - backend is authoritative
    return errors;
  }, [schema, selectedSchemaObj]);

  const isValid = validationErrors.length === 0;

  // Build query object
  const query = useMemo(() => {
    if (!schema || !queryState || !selectedSchemaObj) {
      return {};
    }

    const {
      queryFields = [],
      fieldValues = {},
      rangeFilters = {},
      rangeSchemaFilter = {},
      filters = [],
      orderBy
    } = queryState;
    
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
    if (schemaIsRange) {
      const possibleRangeKey = providedRangeKey
        || selectedSchemaObj?.schema_type?.Range?.range_key
        || selectedSchemaObj?.range_key
        || (selectedSchemaObj?.fields
          ? Object.entries(selectedSchemaObj.fields).find(([, field]) => field?.field_type === 'Range')?.[0]
          : null);
      const activeRangeFilter = rangeSchemaFilter && Object.keys(rangeSchemaFilter).length > 0
        ? rangeSchemaFilter
        : Object.values(rangeFilters).find(filter => filter && typeof filter === 'object' && (filter.key || filter.keyPrefix || (filter.start && filter.end))) || {};

      if (activeRangeFilter.key) {
        builtQuery.rangeKey = activeRangeFilter.key;
      } else if (activeRangeFilter.keyPrefix) {
        builtQuery.rangeKey = activeRangeFilter.keyPrefix;
      }

      if (possibleRangeKey) {
        let filterType = null;
        let filterValue = null;

        if (activeRangeFilter.key) {
          filterType = 'Key';
          filterValue = activeRangeFilter.key;
        } else if (activeRangeFilter.keyPrefix) {
          filterType = 'KeyPrefix';
          filterValue = activeRangeFilter.keyPrefix;
        } else if (activeRangeFilter.start && activeRangeFilter.end) {
          filterType = 'KeyRange';
          filterValue = { start: activeRangeFilter.start, end: activeRangeFilter.end };
        }

        if (filterType && filterValue) {
          builtQuery.filter = {
            range_filter: {
              [possibleRangeKey]: {
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