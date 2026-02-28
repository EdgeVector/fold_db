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
import { useAppSelector, useAppDispatch } from '../store/hooks';
import { selectApprovedSchemas, selectAllSchemas, selectFetchLoading, fetchSchemas } from '../store/schemaSlice';
import { isHashRangeSchema, isRangeSchema, getRangeKey } from '../utils/rangeSchemaHelpers.js';

/**
 * @typedef {Object} QueryState
 * @property {string} selectedSchema - Currently selected schema name
 * @property {string[]} queryFields - Array of selected field names
 * @property {Object} rangeFilters - Range field filters for regular schemas
 * @property {Object} rangeSchemaFilter - Range filters for range schemas
 * @property {string} rangeKeyValue - Current range key value
 * @property {string} hashKeyValue - Current hash key value for HashRange schemas
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
 * @property {Function} refetchSchemas - Force refetch schemas from backend
 * @property {Object[]} approvedSchemas - Filtered approved schemas from Redux
 * @property {boolean} schemasLoading - Loading state for schemas
 * @property {Object|null} selectedSchemaObj - Full selected schema object
 * @property {boolean} isRangeSchema - Whether selected schema is range schema
 * @property {string|null} rangeKey - Range key for selected schema
 */

// Range schema utility functions now imported from rangeSchemaHelpers.js

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
  // Redux state and dispatch - following SchemaTab.jsx pattern (lines 16-21)
  const dispatch = useAppDispatch();
  const schemas = useAppSelector(selectAllSchemas);
  const schemasLoading = useAppSelector(selectFetchLoading);

  // Local state management
  const [selectedSchema, setSelectedSchema] = useState('');
  const [queryFields, setQueryFields] = useState([]);
  const [fieldValues, setFieldValues] = useState({});
  const [rangeFilters, setRangeFilters] = useState({});
  const [rangeKeyValue, setRangeKeyValue] = useState('');
  const [hashKeyValue, setHashKeyValue] = useState('');
  const [rangeSchemaFilter, setRangeSchemaFilter] = useState({});

  // Approved schemas from Redux selector (SCHEMA-002 compliant)
  const approvedSchemas = useAppSelector(selectApprovedSchemas);

  // Memoized selected schema object
  const selectedSchemaObj = useMemo(() => {
    return selectedSchema ? (schemas || []).find(s => s.name === selectedSchema) : null;
  }, [selectedSchema, schemas]);

  // Memoized schema type checks
  const isCurrentSchemaRangeSchema = useMemo(() => {
    return selectedSchemaObj ? isRangeSchema(selectedSchemaObj) : false;
  }, [selectedSchemaObj]);

  const isCurrentSchemaHashRangeSchema = useMemo(() => {
    return selectedSchemaObj ? isHashRangeSchema(selectedSchemaObj) : false;
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
      const selectedSchemaObj = (schemas || []).find(s => s.name === schemaName);
      // Handle both regular schemas (fields array) and transform schemas (transform_fields object)
      const schemaFields = selectedSchemaObj?.fields || selectedSchemaObj?.transform_fields || [];
      const allFieldNames = Array.isArray(schemaFields) 
        ? schemaFields 
        : Object.keys(schemaFields);
      setQueryFields(allFieldNames);
      
      // Initialize fieldValues with empty strings for all fields
      const initialFieldValues = {};
      allFieldNames.forEach(fieldName => {
        initialFieldValues[fieldName] = '';
      });
      setFieldValues(initialFieldValues);
    } else {
      setQueryFields([]);
      setFieldValues({});
    }
    
    // Clear filters when schema changes
    setRangeFilters({});
    setRangeKeyValue('');
    setHashKeyValue('');
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
    
    // Update fieldValues when fields are toggled
    setFieldValues(prev => {
      if (prev[fieldName] !== undefined) {
        return prev; // Field already has a value, keep it
      }
      return {
        ...prev,
        [fieldName]: '' // Initialize with empty string for new fields
      };
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
   * Handle field value changes
   */
  const handleFieldValueChange = useCallback((fieldName, value) => {
    setFieldValues(prev => ({
      ...prev,
      [fieldName]: value
    }));
  }, []);

  /**
   * Clear all query state
   */
  const clearState = useCallback(() => {
    setSelectedSchema('');
    setQueryFields([]);
    setFieldValues({});
    setRangeFilters({});
    setRangeKeyValue('');
    setHashKeyValue('');
    setRangeSchemaFilter({});
  }, []);

  /**
   * Force refetch schemas from backend
   */
  const refetchSchemas = useCallback(() => {
    dispatch(fetchSchemas({ forceRefresh: true }));
  }, [dispatch]);

  // Aggregate state object
  const state = {
    selectedSchema,
    queryFields,
    fieldValues,
    rangeFilters,
    rangeSchemaFilter,
    rangeKeyValue,
    hashKeyValue
  };

  return {
    state,
    setSelectedSchema,
    setQueryFields,
    setFieldValues,
    toggleField,
    handleFieldValueChange,
    setRangeFilters,
    setRangeSchemaFilter,
    setRangeKeyValue,
    setHashKeyValue,
    clearState,
    clearQuery: clearState,
    handleSchemaChange,
    handleRangeFilterChange,
    refetchSchemas,
    approvedSchemas,
    schemasLoading,
    selectedSchemaObj,
    isRangeSchema: isCurrentSchemaRangeSchema,
    isHashRangeSchema: isCurrentSchemaHashRangeSchema,
    rangeKey
  };
}

export default useQueryState;
export { useQueryState };