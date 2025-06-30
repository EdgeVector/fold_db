/**
 * QueryBuilder Component
 * Handles query construction logic with Redis schema integration
 * Part of UCR-1-3: Create QueryBuilder component with Redux schema integration
 * Uses Redux schema state and authentication from existing store
 */

import { useMemo } from 'react';
import { SCHEMA_STATES, SCHEMA_ERROR_MESSAGES } from '../../constants/redux.js';

/**
 * @typedef {Object} QueryBuilderProps
 * @property {Object} queryState - Current query state from useQueryState
 * @property {Object} selectedSchemaObj - Full selected schema object
 * @property {boolean} isRangeSchema - Whether selected schema is range schema
 * @property {string|null} rangeKey - Range key for selected schema
 */

/**
 * @typedef {Object} QueryBuilderResult
 * @property {Object|null} query - Built query object ready for API
 * @property {string[]} validationErrors - Array of validation error messages
 * @property {boolean} isValid - Whether the query is valid for execution
 * @property {function} buildQuery - Function to manually build query
 * @property {function} validateQuery - Function to validate query
 */

/**
 * Range schema utility functions
 */
const isRangeSchema = (schema) => {
  return schema?.fields && Object.values(schema.fields).some(field => field.field_type === 'Range');
};

const getRangeKey = (schema) => {
  if (!schema?.fields) return null;
  const rangeField = Object.entries(schema.fields).find(([, field]) => field.field_type === 'Range');
  return rangeField ? rangeField[0] : null;
};

/**
 * QueryBuilder component for constructing validated queries
 * 
 * Handles the query building logic extracted from QueryTab component,
 * using Redux schema state for consistent data access.
 * 
 * @param {QueryBuilderProps} props
 * @returns {QueryBuilderResult}
 */
function useQueryBuilder({
  queryState,
  selectedSchemaObj,
  isRangeSchema: isCurrentSchemaRangeSchema,
  rangeKey
}) {
  /**
   * Validate the current query state
   */
  const validationErrors = useMemo(() => {
    const errors = [];

    // Basic validation
    if (!queryState.selectedSchema) {
      errors.push('Schema selection is required');
    }

    if (!queryState.queryFields || queryState.queryFields.length === 0) {
      errors.push('At least one field must be selected');
    }

    // Range schema validation
    if (isCurrentSchemaRangeSchema && queryState.rangeSchemaFilter) {
      const filter = queryState.rangeSchemaFilter;
      if (filter.start && filter.end && filter.start >= filter.end) {
        errors.push('Start key must be less than end key');
      }
    }

    // Range field validation for regular schemas
    if (!isCurrentSchemaRangeSchema && selectedSchemaObj?.fields) {
      const selectedSchemaFields = selectedSchemaObj.fields;
      const rangeFieldsWithFilters = queryState.queryFields.filter(fieldName => {
        const field = selectedSchemaFields[fieldName];
        return field?.field_type === 'Range' && queryState.rangeFilters[fieldName];
      });

      rangeFieldsWithFilters.forEach(fieldName => {
        const filter = queryState.rangeFilters[fieldName];
        if (filter.start && filter.end && filter.start >= filter.end) {
          errors.push(`Range field ${fieldName}: Start key must be less than end key`);
        }
      });
    }

    return errors;
  }, [queryState, selectedSchemaObj, isCurrentSchemaRangeSchema]);

  /**
   * Check if query is valid
   */
  const isValid = useMemo(() => {
    return validationErrors.length === 0 && 
           queryState.selectedSchema && 
           queryState.queryFields.length > 0;
  }, [validationErrors, queryState]);

  /**
   * Build the query object following QueryTab.jsx pattern (lines 87-170)
   */
  const buildQuery = useMemo(() => {
    if (!isValid) return null;

    let query = {
      type: 'query',
      schema: queryState.selectedSchema,
      fields: queryState.queryFields
    };

    // Handle range schema queries
    if (isCurrentSchemaRangeSchema && selectedSchemaObj) {
      const rangeFilter = queryState.rangeSchemaFilter;
      
      if (rangeFilter.start && rangeFilter.end) {
        query.filter = {
          range_filter: {
            [getRangeKey(selectedSchemaObj)]: {
              KeyRange: {
                start: rangeFilter.start,
                end: rangeFilter.end
              }
            }
          }
        };
      } else if (rangeFilter.key) {
        query.filter = {
          range_filter: {
            [getRangeKey(selectedSchemaObj)]: rangeFilter.key
          }
        };
      } else if (rangeFilter.keyPrefix) {
        query.filter = {
          range_filter: {
            [getRangeKey(selectedSchemaObj)]: {
              KeyPrefix: rangeFilter.keyPrefix
            }
          }
        };
      }
    } else if (selectedSchemaObj?.fields) {
      // Handle regular schema range field filters
      const selectedSchemaFields = selectedSchemaObj.fields;
      const rangeFieldsWithFilters = queryState.queryFields.filter(fieldName => {
        const field = selectedSchemaFields[fieldName];
        return field?.field_type === 'Range' && queryState.rangeFilters[fieldName];
      });

      if (rangeFieldsWithFilters.length > 0) {
        const fieldName = rangeFieldsWithFilters[0]; // Support one range filter for now
        const filter = queryState.rangeFilters[fieldName];
        
        if (filter.start && filter.end) {
          query.filter = {
            field: fieldName,
            range_filter: {
              KeyRange: {
                start: filter.start,
                end: filter.end
              }
            }
          };
        } else if (filter.key) {
          query.filter = {
            field: fieldName,
            range_filter: {
              Key: filter.key
            }
          };
        } else if (filter.keyPrefix) {
          query.filter = {
            field: fieldName,
            range_filter: {
              KeyPrefix: filter.keyPrefix
            }
          };
        }
      }
    }

    return query;
  }, [queryState, selectedSchemaObj, isCurrentSchemaRangeSchema, isValid]);

  /**
   * Manual query building function
   */
  const buildQueryManually = () => {
    return buildQuery;
  };

  /**
   * Manual validation function
   */
  const validateQuery = () => {
    return {
      isValid,
      errors: validationErrors
    };
  };

  return {
    query: buildQuery,
    validationErrors,
    isValid,
    buildQuery: buildQueryManually,
    validateQuery
  };
}

/**
 * QueryBuilder component wrapper for use in JSX
 * 
 * @param {QueryBuilderProps & { children: function }} props
 * @returns {JSX.Element}
 */
function QueryBuilder({ children, ...props }) {
  const queryBuilder = useQueryBuilder(props);
  
  if (typeof children === 'function') {
    return children(queryBuilder);
  }

  return null;
}

export default QueryBuilder;
export { useQueryBuilder, QueryBuilder };