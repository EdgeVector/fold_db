/**
 * Custom hook for managing approved schemas with SCHEMA-002 compliance
 * TASK-003: Updated to use Redux state management instead of local state
 * Centralizes schema fetching logic and enforces access control
 */

import { useEffect, useCallback } from 'react';
import { useAppSelector, useAppDispatch } from '../store/hooks';
import {
  fetchSchemas,
  selectApprovedSchemas,
  selectAllSchemas,
  selectFetchLoading,
  selectFetchError,
  selectCacheInfo
} from '../store/schemaSlice';
import {
  SCHEMA_STATES
} from '../constants/schemas.js';

/**
 * Hook for managing approved schemas with caching and retry logic
 * TASK-003: Now uses Redux state management for centralized schema state
 *
 * @returns {Object} Hook result object
 * @returns {Array} approvedSchemas - Array of schemas with state 'approved'
 * @returns {boolean} isLoading - Loading state indicator
 * @returns {string|null} error - Error message if any
 * @returns {Function} refetch - Function to manually refetch schemas
 * @returns {Function} getSchemaByName - Get specific schema by name
 * @returns {Function} isSchemaApproved - Check if schema is approved
 */
export function useApprovedSchemas() {
  // Redux state and dispatch
  const dispatch = useAppDispatch();
  const approvedSchemas = useAppSelector(selectApprovedSchemas);
  const allSchemas = useAppSelector(selectAllSchemas);
  const isLoading = useAppSelector(selectFetchLoading);
  const error = useAppSelector(selectFetchError);
  const cacheInfo = useAppSelector(selectCacheInfo);

  /**
   * Normalizes schema state to lowercase string
   * @param {*} state - Schema state in various formats
   * @returns {string} Normalized state string
   */
  const normalizeState = useCallback((state) => {
    if (typeof state === 'string') return state.toLowerCase();
    if (typeof state === 'object' && state !== null) return String(state).toLowerCase();
    return String(state || '').toLowerCase();
  }, []);

  /**
   * Manual refetch function that bypasses cache
   */
  const refetch = useCallback(async () => {
    // Force refresh by dispatching with forceRefresh: true
    dispatch(fetchSchemas({ forceRefresh: true }));
  }, [dispatch]);

  /**
   * Get specific schema by name
   * @param {string} name - Schema name
   * @returns {Object|null} Schema object or null if not found
   */
  const getSchemaByName = useCallback((name) => {
    return allSchemas.find(schema => schema.name === name) || null;
  }, [allSchemas]);

  /**
   * Check if a schema is approved (SCHEMA-002 compliance)
   * @param {string} name - Schema name
   * @returns {boolean} True if schema is approved
   */
  const isSchemaApproved = useCallback((name) => {
    const schema = getSchemaByName(name);
    if (!schema) return false;
    
    const normalizedState = normalizeState(schema.state);
    return normalizedState === SCHEMA_STATES.APPROVED;
  }, [getSchemaByName, normalizeState]);

  // Initial fetch on mount if cache is invalid
  useEffect(() => {
    if (!cacheInfo.isValid) {
      dispatch(fetchSchemas());
    }
  }, [dispatch, cacheInfo.isValid]);

  return {
    approvedSchemas,
    isLoading,
    error,
    refetch,
    getSchemaByName,
    isSchemaApproved,
    // Additional utility for components that need all schemas for display
    allSchemas
  };
}

export default useApprovedSchemas;