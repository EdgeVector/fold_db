/**
 * @fileoverview Custom hook for managing query state with Redux integration
 *
 * This hook provides centralized query state management, following the established
 * patterns from useApprovedSchemas.js. It handles query field selection, schema
 * management, and filter state for the QueryTab component.
 *
 * Part of UCR-1-2: Extract custom hooks for query state management with Redux integration
 * Follows patterns established in useApprovedSchemas.js
 *
 * @module useQueryState
 * @since 2.0.0
 */

import { useState, useCallback, useMemo } from 'react';
import { useAppSelector } from '../store/hooks';
import { selectAllSchemas, selectFetchLoading } from '../store/schemaSlice';
import { SCHEMA_STATES } from '../constants/redux.js';

/**
 * @typedef {Object} QueryState
 * @property {string} selectedSchema - Currently selected schema name
 * @property {string[]} queryFields - Array of selected field names
 * @property {Object} rangeFilters - Range field filters for regular schemas
 * @property {Object} rangeSchemaFilter - Range filters for range schemas
 * @property {string} rangeKeyValue - Current range key value
 */

/**
 * @typedef {Object} UseQueryStateResult
 * @property {QueryState} state - Current query state
 * @property {Function} setSelectedSchema - Set the selected schema
 * @property {Function} setQueryFields - Set the selected query fields
 * @property {Function} toggleField - Toggle a field in the selection
 * @property {Function} setRangeFilters - Set range filters for regular schemas
 * @property {Function} setRangeSchemaFilter - Set range filters for range schemas
 * @property {Function} setRangeKeyValue - Set range key value
 * @property {Function} clearState - Clear all query state
 * @property {Function} handleSchemaChange - Handle schema selection change
 * @property {Function} handleRangeFilterChange - Handle range filter changes
 * @property {Object[]} approvedSchemas - Filtered approved schemas from Redux
 * @property {boolean} schemasLoading - Loading state for schemas
 * @property {Object|null} selectedSchemaObj - Full selected schema object
 * @property {boolean} isRangeSchema - Whether selected schema is range schema
 * @property {string|null} rangeKey - Range key for selected schema
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
 * Custom hook for managing query state with Redux integration
 * 
 * Provides centralized state management for query operations following
 * established patterns from useApprovedSchemas and SchemaTab components.
 * 
 * @returns {UseQueryStateResult} Query state and management functions
 * 
 * @example
 * function QueryComponent() {
 *   const {
 *     state,
 *     setSelectedSchema,
 *     toggleField,
 *     approvedSchemas,
 *     handleSchemaChange
 *   } = useQueryState();
 * 
 *   return (
 *     <SelectField
 *       value={state.selectedSchema}
 *       onChange={handleSchemaChange}
 *       options={approvedSchemas}
 *     />
 *   );
 * }
 */
function useQueryState() {
  // Authentication check - prevent accessing schemas without proper auth
  const authState = useAppSelector(state => state.auth);
  const { isAuthenticated } = authState;

  // Redux state - following SchemaTab.jsx pattern (lines 16-21)
  // Only access schemas if authenticated to prevent auth errors
  const schemas = useAppSelector(selectAllSchemas);
  const schemasLoading = useAppSelector(selectFetchLoading);

  // Local state management
  const [selectedSchema, setSelectedSchema] = useState('');
  const [queryFields, setQueryFields] = useState([]);
  const [rangeFilters, setRangeFilters] = useState({});
  const [rangeKeyValue, setRangeKeyValue] = useState('');
  const [rangeSchemaFilter, setRangeSchemaFilter] = useState({});

  // Memoized approved schemas - following QueryTab.jsx pattern (lines 265-271)
  // Return empty array if not authenticated to prevent accessing unauthenticated data
  const approvedSchemas = useMemo(() => {
    if (!isAuthenticated) {
      return [];
    }
    return schemas.filter(schema => {
      const state = typeof schema.state === 'string'
        ? schema.state.toLowerCase()
        : String(schema.state || '').toLowerCase();
      return state === SCHEMA_STATES.APPROVED;
    });
  }, [schemas, isAuthenticated]);

  // Memoized selected schema object - only return if authenticated
  const selectedSchemaObj = useMemo(() => {
    if (!isAuthenticated) {
      return null;
    }
    return selectedSchema ? schemas.find(s => s.name === selectedSchema) : null;
  }, [selectedSchema, schemas, isAuthenticated]);

  // Memoized schema type checks
  const isCurrentSchemaRangeSchema = useMemo(() => {
    return selectedSchemaObj ? isRangeSchema(selectedSchemaObj) : false;
  }, [selectedSchemaObj]);

  const rangeKey = useMemo(() => {
    return selectedSchemaObj ? getRangeKey(selectedSchemaObj) : null;
  }, [selectedSchemaObj]);

  /**
   * Handle schema selection change
   * Follows QueryTab.jsx handleSchemaChange pattern (lines 41-58)
   */
  const handleSchemaChange = useCallback((schemaName) => {
    setSelectedSchema(schemaName);
    
    // Default to all fields being checked when a schema is selected
    if (schemaName) {
      const selectedSchemaObj = schemas.find(s => s.name === schemaName);
      const allFieldNames = selectedSchemaObj?.fields ? Object.keys(selectedSchemaObj.fields) : [];
      setQueryFields(allFieldNames);
    } else {
      setQueryFields([]);
    }
    
    // Clear filters when schema changes
    setRangeFilters({});
    setRangeKeyValue('');
    setRangeSchemaFilter({});
  }, [schemas]);

  /**
   * Toggle field selection
   * Follows QueryTab.jsx handleFieldToggle pattern (lines 60-67)
   */
  const toggleField = useCallback((fieldName) => {
    setQueryFields(prev => {
      if (prev.includes(fieldName)) {
        return prev.filter(f => f !== fieldName);
      }
      return [...prev, fieldName];
    });
  }, []);

  /**
   * Handle range filter changes for regular schemas
   * Follows QueryTab.jsx handleRangeFilterChange pattern (lines 69-77)
   */
  const handleRangeFilterChange = useCallback((fieldName, filterType, value) => {
    setRangeFilters(prev => ({
      ...prev,
      [fieldName]: {
        ...prev[fieldName],
        [filterType]: value
      }
    }));
  }, []);

  /**
   * Clear all query state
   */
  const clearState = useCallback(() => {
    setSelectedSchema('');
    setQueryFields([]);
    setRangeFilters({});
    setRangeKeyValue('');
    setRangeSchemaFilter({});
  }, []);

  // Aggregate state object
  const state = {
    selectedSchema,
    queryFields,
    rangeFilters,
    rangeSchemaFilter,
    rangeKeyValue
  };

  return {
    state,
    setSelectedSchema,
    setQueryFields,
    toggleField,
    setRangeFilters,
    setRangeSchemaFilter,
    setRangeKeyValue,
    clearState,
    handleSchemaChange,
    handleRangeFilterChange,
    approvedSchemas,
    schemasLoading,
    selectedSchemaObj,
    isRangeSchema: isCurrentSchemaRangeSchema,
    rangeKey
  };
}

export default useQueryState;
export { useQueryState };