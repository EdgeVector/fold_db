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
import type { Schema } from '@generated/generated';

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

  // No frontend validation - backend is authoritative
  const validationErrors = useMemo(() => {
    return [];
  }, []);

  const isValid = true; // Always valid - backend validates

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

    // Add range schema filter for range schemas
    if (schemaIsRange) {
      const activeRangeFilter = rangeSchemaFilter && Object.keys(rangeSchemaFilter).length > 0
        ? rangeSchemaFilter
        : Object.values(rangeFilters).find(filter => filter && typeof filter === 'object' && (filter.key || filter.keyPrefix || (filter.start && filter.end))) || {};
      
      // Handle direct rangeKey from queryState
      const directRangeKey = queryState?.rangeKeyValue;
      if (!activeRangeFilter.key && !activeRangeFilter.keyPrefix && !(activeRangeFilter.start && activeRangeFilter.end) && directRangeKey) {
        activeRangeFilter.key = directRangeKey;
      }

      // Create filter if any range filter value exists
      if (activeRangeFilter.key) {
        builtQuery.filter = createHashKeyFilter(activeRangeFilter.key);
      } else if (activeRangeFilter.keyPrefix) {
        builtQuery.filter = createRangePrefixFilter(activeRangeFilter.keyPrefix);
      } else if (activeRangeFilter.start && activeRangeFilter.end) {
        builtQuery.filter = createRangeRangeFilter(activeRangeFilter.start, activeRangeFilter.end);
      }
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