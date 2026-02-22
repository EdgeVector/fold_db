/**
 * QueryBuilder Component
 * Handles query construction logic with Redis schema integration
 * Part of UCR-1-3: Create QueryBuilder component with Redux schema integration
 * Uses Redux schema state and authentication from existing store
 */

import { useMemo } from 'react';
import { useQueryBuilder } from '../../hooks/useQueryBuilder';

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
 * Range schema utility functions for declarative schemas
 */
const detectRangeSchema = (schema) => {
  return schema?.schema_type === 'Range';
};

const extractRangeKey = (schema) => {
  return schema?.key?.range_field || null;
};

/**
 * QueryBuilder component wrapper for use in JSX
 * 
 * @param {QueryBuilderProps & { children: function }} props
 * @returns {JSX.Element}
 */
function QueryBuilder({
  children,
  queryState,
  schemas,
  selectedSchemaObj,
  isRangeSchema,
  rangeKey,
  schema,
  ...rest
}) {
  const resolvedSchema = useMemo(() => {
    if (schema) {
      return schema;
    }

    if (queryState?.selectedSchema) {
      return queryState.selectedSchema;
    }

    return selectedSchemaObj?.name ?? null;
  }, [schema, queryState?.selectedSchema, selectedSchemaObj?.name]);

  const resolvedSchemaObj = useMemo(() => {
    if (selectedSchemaObj) {
      return selectedSchemaObj;
    }

    if (resolvedSchema && schemas && schemas[resolvedSchema]) {
      return schemas[resolvedSchema];
    }

    return null;
  }, [resolvedSchema, schemas, selectedSchemaObj]);

  const resolvedSchemas = useMemo(() => {
    if (schemas) {
      return schemas;
    }

    if (resolvedSchema && resolvedSchemaObj) {
      return { [resolvedSchema]: resolvedSchemaObj };
    }

    return undefined;
  }, [schemas, resolvedSchema, resolvedSchemaObj]);

  const resolvedIsRangeSchema = useMemo(() => {
    if (typeof isRangeSchema === 'boolean') {
      return isRangeSchema;
    }

    return detectRangeSchema(resolvedSchemaObj);
  }, [isRangeSchema, resolvedSchemaObj]);

  const resolvedRangeKey = useMemo(() => {
    if (rangeKey) {
      return rangeKey;
    }

    return extractRangeKey(resolvedSchemaObj);
  }, [rangeKey, resolvedSchemaObj]);

  const hookArguments = useMemo(() => ({
    ...rest,
    schema: resolvedSchema,
    queryState,
    schemas: resolvedSchemas,
    selectedSchemaObj: resolvedSchemaObj,
    isRangeSchema: resolvedIsRangeSchema,
    rangeKey: resolvedRangeKey
  }), [rest, resolvedSchema, queryState, resolvedSchemas, resolvedSchemaObj, resolvedIsRangeSchema, resolvedRangeKey]);

  const queryBuilder = useQueryBuilder(hookArguments);

  if (typeof children === 'function') {
    return children(queryBuilder);
  }

  return null;
}

export default QueryBuilder;
// eslint-disable-next-line react-refresh/only-export-components
export { useQueryBuilder, QueryBuilder };
