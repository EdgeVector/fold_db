/**
 * @fileoverview Custom hook for managing approved schemas with SCHEMA-002 compliance
 *
 * This hook provides a centralized interface for accessing and managing approved schemas,
 * enforcing SCHEMA-002 compliance by only allowing mutations and queries on approved schemas.
 * It integrates with Redux state management for consistent state across the application.
 *
 * TASK-003: Updated to use Redux state management instead of local state
 * TASK-006: Enhanced with comprehensive JSDoc documentation
 *
 * @module useApprovedSchemas
 * @since 2.0.0
 * @see {@link https://github.com/shiba4life/fold_db/docs/project_logic.md#schema-002} SCHEMA-002 compliance documentation
 */

import { useEffect, useCallback } from "react";
import { useAppSelector, useAppDispatch } from "../store/hooks";
import {
  fetchSchemas,
  selectApprovedSchemas,
  selectAllSchemas,
  selectFetchLoading,
  selectFetchError,
  selectCacheInfo,
} from "../store/schemaSlice";
import { SCHEMA_STATES } from "../constants/redux.js";
import { normalizeSchemaState } from "../utils/rangeSchemaHelpers.js";

/**
 * @typedef {Object} Schema
 * @property {string} name - Unique schema identifier
 * @property {string} state - Schema state (available|approved|blocked)
 * @property {Object} fields - Schema field definitions
 * @property {Object} [schema_type] - Schema type information for range schemas
 * @property {Object} [rangeInfo] - Range schema specific information
 */

/**
 * @typedef {Object} UseApprovedSchemasResult
 * @property {Schema[]} approvedSchemas - Array of schemas with state 'approved'
 * @property {boolean} isLoading - Loading state indicator for schema fetching
 * @property {string|null} error - Error message if schema fetching fails
 * @property {Function} refetch - Function to manually refetch schemas from API
 * @property {Function} getSchemaByName - Get specific schema by name from all schemas
 * @property {Function} isSchemaApproved - Check if a schema is approved for operations
 * @property {Schema[]} allSchemas - All available schemas regardless of state
 */

/**
 * Custom hook for managing approved schemas with SCHEMA-002 compliance
 *
 * This hook provides a centralized interface for accessing approved schemas while
 * enforcing SCHEMA-002 compliance rules. It uses Redux for state management and
 * provides caching, retry logic, and automatic updates.
 *
 * **Key Features:**
 * - SCHEMA-002 compliance enforcement
 * - Redux-based state management
 * - Automatic caching with TTL
 * - Retry logic for failed requests
 * - Real-time state updates
 * - Type-safe schema access
 *
 * **Usage Examples:**
 * ```jsx
 * // Basic usage
 * const { approvedSchemas, isLoading, error } = useApprovedSchemas();
 *
 * // Check if specific schema is approved
 * const { isSchemaApproved } = useApprovedSchemas();
 * if (isSchemaApproved('user_profiles')) {
 *   // Safe to perform mutations/queries
 * }
 *
 * // Manual refresh
 * const { refetch } = useApprovedSchemas();
 * await refetch();
 * ```
 *
 * **SCHEMA-002 Compliance:**
 * This hook enforces SCHEMA-002 rules by:
 * - Only returning schemas in 'approved' state for operations
 * - Providing validation functions for schema state checking
 * - Integrating with Redux store that manages schema state transitions
 *
 * @function useApprovedSchemas
 * @returns {UseApprovedSchemasResult} Hook result object with approved schemas and utility functions
 *
 * @example
 * ```jsx
 * function MyComponent() {
 *   const {
 *     approvedSchemas,
 *     isLoading,
 *     error,
 *     isSchemaApproved,
 *     refetch
 *   } = useApprovedSchemas();
 *
 *   if (isLoading) return <div>Loading schemas...</div>;
 *   if (error) return <div>Error: {error}</div>;
 *
 *   return (
 *     <div>
 *       <h2>Approved Schemas ({approvedSchemas.length})</h2>
 *       {approvedSchemas.map(schema => (
 *         <div key={schema.name}>
 *           {schema.name} - {isSchemaApproved(schema.name) ? 'Ready' : 'Not Ready'}
 *         </div>
 *       ))}
 *     </div>
 *   );
 * }
 * ```
 *
 * @since 2.0.0
 */
export function useApprovedSchemas({ enabled = true } = {}) {
  // Redux state and dispatch
  const dispatch = useAppDispatch();
  const approvedSchemas = useAppSelector(selectApprovedSchemas);
  const allSchemas = useAppSelector(selectAllSchemas);
  const isLoading = useAppSelector(selectFetchLoading);
  const error = useAppSelector(selectFetchError);
  const cacheInfo = useAppSelector(selectCacheInfo);

  // Using consolidated schema state normalization from rangeSchemaHelpers.js
  // This eliminates duplication of state normalization logic

  /**
   * Manual refetch function that bypasses cache
   */
  const refetch = useCallback(async () => {
    // Only refetch if enabled
    if (enabled) {
      // Force refresh by dispatching with forceRefresh: true
      dispatch(fetchSchemas({ forceRefresh: true }));
    }
  }, [dispatch, enabled]);

  /**
   * Get specific schema by name
   * @param {string} name - Schema name
   * @returns {Object|null} Schema object or null if not found
   */
  const getSchemaByName = useCallback(
    (name) => {
      return allSchemas.find((schema) => schema.name === name) || null;
    },
    [allSchemas],
  );

  /**
   * Check if a schema is approved (SCHEMA-002 compliance)
   * @param {string} name - Schema name
   * @returns {boolean} True if schema is approved
   */
  const isSchemaApproved = useCallback(
    (name) => {
      const schema = getSchemaByName(name);
      if (!schema) return false;

      // Use the consolidated normalization function for consistency
      const normalizedState = normalizeSchemaState(schema.state);
      return normalizedState === SCHEMA_STATES.APPROVED;
    },
    [getSchemaByName],
  );

  // Initial fetch on mount if cache is invalid AND enabled
  useEffect(() => {
    if (enabled && !cacheInfo.isValid) {
      dispatch(fetchSchemas());
    }
  }, [dispatch, enabled, cacheInfo.isValid]); // Fetch when enabled changes to true or cache becomes invalid

  return {
    approvedSchemas,
    isLoading,
    error,
    refetch,
    getSchemaByName,
    isSchemaApproved,
    // Additional utility for components that need all schemas for display
    allSchemas,
  };
}

export default useApprovedSchemas;
