/**
 * useQueryBuilder Hook
 * Handles query building logic with validation
 * Part of UCR-1-5: Create QueryBuilder hook for complex query construction
 */

import { useMemo, useCallback } from 'react';
import { useAppSelector } from '../store/hooks';
import { selectApprovedSchemas } from '../store/schemaSlice';
import { isHashRangeSchema, isRangeSchema as detectRangeSchema } from '../utils/rangeSchemaHelpers.js';
import { 
  createFilterFromRangeInput, 
  createHashKeyFilter, 
  createRangePrefixFilter, 
  createRangeRangeFilter,
  type HashRangeFilter,
  type RangeFilterInput
} from '../utils/filterUtils';
import type { Schema } from '../types/generated';

interface QueryState {
  queryFields?: string[];
  fieldValues?: Record<string, any>;
  rangeFilters?: Record<string, RangeFilterInput>;
  rangeSchemaFilter?: RangeFilterInput;
  hashKeyValue?: string;
  rangeKeyValue?: string;
  filters?: any[];
  orderBy?: any;
}

interface UseQueryBuilderOptions {
  schema?: string;
  queryState?: QueryState;
  schemas?: Record<string, Schema>;
  selectedSchemaObj?: Schema;
  isRangeSchema?: boolean;
  rangeKey?: string;
}

interface QueryBuilderResult {
  query: {
    schema_name?: string;
    fields?: string[];
    filter?: HashRangeFilter;
  };
  isValid: boolean;
  validationErrors: string[];
}

/**
 * Query builder hook that handles query construction and validation
 */
export function useQueryBuilder({
  schema,
  queryState,
  schemas,
  selectedSchemaObj: providedSelectedSchema,
  isRangeSchema: providedIsRangeSchema,
  rangeKey: providedRangeKey
}: UseQueryBuilderOptions): QueryBuilderResult {
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
    
    // Build query object that matches backend Query struct exactly
    const builtQuery: {
      schema_name: string;
      fields: string[];
      filter?: any;
    } = {
      schema_name: schema, // Backend expects schema_name, not schema
      fields: queryFields, // Array of selected field names
    };

    // Handle HashRange schema queries
    if (isHashRangeSchema(selectedSchemaObj)) {
      const hashKey = queryState.hashKeyValue;
      const rangeKey = queryState.rangeSchemaFilter?.key;
      
      if (hashKey && hashKey.trim()) {
        // For HashRange schemas, use HashKey filter for hash key filtering
        builtQuery.filter = createHashKeyFilter(hashKey.trim());
      } else if (rangeKey && rangeKey.trim()) {
        // For HashRange schemas, use HashKey filter for range key filtering
        builtQuery.filter = createHashKeyFilter(rangeKey.trim());
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
      
      // Handle direct rangeKey from queryState (fallback for when rangeKey is set directly)
      const directRangeKey = queryState?.rangeKeyValue;
      if (!activeRangeFilter.key && !activeRangeFilter.keyPrefix && !(activeRangeFilter.start && activeRangeFilter.end) && directRangeKey) {
        activeRangeFilter.key = directRangeKey;
      }

      // Note: We don't set builtQuery.rangeKey because the backend doesn't recognize this field
      // The backend only processes the 'filter' field, which is set below

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
          // Use type-safe filter creation utilities
          const rangeInput: RangeFilterInput = {
            key: activeRangeFilter.key,
            keyPrefix: activeRangeFilter.keyPrefix,
            start: activeRangeFilter.start,
            end: activeRangeFilter.end
          };
          
          const filter = createFilterFromRangeInput(rangeInput);
          if (filter) {
            builtQuery.filter = filter;
          }
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
    isValid
  };
}

export default useQueryBuilder;