/**
 * Custom hook for managing approved schemas with SCHEMA-002 compliance
 * Centralizes schema fetching logic and enforces access control
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import { 
  SCHEMA_FETCH_RETRY_COUNT,
  SCHEMA_CACHE_DURATION_MS,
  SCHEMA_STATES,
  SCHEMA_API_ENDPOINTS,
  VALIDATION_MESSAGES
} from '../constants/schemas.js';

/**
 * Hook for managing approved schemas with caching and retry logic
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
  const [schemas, setSchemas] = useState([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState(null);
  const cacheRef = useRef({ data: null, timestamp: null });
  const retryCountRef = useRef(0);

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
   * Checks if cached data is still valid
   * @returns {boolean} True if cache is valid
   */
  const isCacheValid = useCallback(() => {
    if (!cacheRef.current.data || !cacheRef.current.timestamp) {
      return false;
    }
    
    const now = Date.now();
    const cacheAge = now - cacheRef.current.timestamp;
    return cacheAge < SCHEMA_CACHE_DURATION_MS;
  }, []);

  /**
   * Fetches schemas with retry logic and caching
   */
  const fetchSchemas = useCallback(async () => {
    // Return cached data if valid
    if (isCacheValid()) {
      setSchemas(cacheRef.current.data);
      setIsLoading(false);
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      // Fetch available schemas from filesystem
      const availableResponse = await fetch(SCHEMA_API_ENDPOINTS.AVAILABLE);
      if (!availableResponse.ok) {
        throw new Error(`Failed to fetch available schemas: ${availableResponse.status}`);
      }
      const availableData = await availableResponse.json();
      
      // Fetch persisted schema states from database
      const persistedResponse = await fetch(SCHEMA_API_ENDPOINTS.PERSISTED);
      if (!persistedResponse.ok) {
        throw new Error(`Failed to fetch persisted schemas: ${persistedResponse.status}`);
      }
      const persistedData = await persistedResponse.json();
      
      console.log('📁 Available schemas:', availableData.data || []);
      console.log('🗄️ Persisted schemas:', persistedData.data || {});
      
      const availableSchemas = availableData.data || [];
      const persistedStates = persistedData.data || {};
      
      // Create schemas with states - use persisted state if available, otherwise 'available'
      const schemasWithStates = availableSchemas.map(name => ({
        name,
        state: persistedStates[name] || SCHEMA_STATES.AVAILABLE,
        fields: {} // Will be populated below
      }));
      
      console.log('📋 Merged schemas for UI:', schemasWithStates);
      
      // Fetch detailed schema information for all schemas
      const schemasWithDetails = await Promise.all(
        schemasWithStates.map(async (schema) => {
          try {
            const schemaResponse = await fetch(`${SCHEMA_API_ENDPOINTS.SCHEMA_DETAIL}/${schema.name}`);
            if (schemaResponse.ok) {
              const schemaData = await schemaResponse.json();
              return {
                ...schema,
                ...schemaData, // Include the full schema data including schema_type
                fields: schemaData.fields || {}
              };
            } else {
              console.log(`⚠️ Schema ${schema.name} not loaded in memory (${schemaResponse.status}), keeping basic info`);
            }
          } catch (err) {
            console.log(`⚠️ Failed to fetch details for schema ${schema.name}:`, err.message);
          }
          return schema; // Return original if fetch fails
        })
      );
      
      console.log('✅ Final schemas for UI:', schemasWithDetails);
      
      // Cache the result
      cacheRef.current = {
        data: schemasWithDetails,
        timestamp: Date.now()
      };
      
      setSchemas(schemasWithDetails);
      retryCountRef.current = 0; // Reset retry count on success
    } catch (fetchError) {
      console.error('Failed to fetch schemas:', fetchError);
      
      // Retry logic
      if (retryCountRef.current < SCHEMA_FETCH_RETRY_COUNT) {
        retryCountRef.current += 1;
        console.log(`Retrying schema fetch (attempt ${retryCountRef.current}/${SCHEMA_FETCH_RETRY_COUNT})`);
        
        // Exponential backoff: 1s, 2s, 4s
        const delay = Math.pow(2, retryCountRef.current - 1) * 1000;
        setTimeout(() => {
          fetchSchemas();
        }, delay);
        return;
      }
      
      setError(`Failed to fetch schemas after ${SCHEMA_FETCH_RETRY_COUNT} attempts: ${fetchError.message}`);
      retryCountRef.current = 0; // Reset for next manual retry
    } finally {
      setIsLoading(false);
    }
  }, [isCacheValid]);

  /**
   * Manual refetch function that bypasses cache
   */
  const refetch = useCallback(async () => {
    // Clear cache to force fresh fetch
    cacheRef.current = { data: null, timestamp: null };
    retryCountRef.current = 0;
    await fetchSchemas();
  }, [fetchSchemas]);

  /**
   * Get specific schema by name
   * @param {string} name - Schema name
   * @returns {Object|null} Schema object or null if not found
   */
  const getSchemaByName = useCallback((name) => {
    return schemas.find(schema => schema.name === name) || null;
  }, [schemas]);

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

  // Initial fetch on mount
  useEffect(() => {
    fetchSchemas();
  }, [fetchSchemas]);

  // Filter to only approved schemas (SCHEMA-002 compliance)
  const approvedSchemas = schemas.filter(schema => {
    const normalizedState = normalizeState(schema.state);
    return normalizedState === SCHEMA_STATES.APPROVED;
  });

  return {
    approvedSchemas,
    isLoading,
    error,
    refetch,
    getSchemaByName,
    isSchemaApproved,
    // Additional utility for components that need all schemas for display
    allSchemas: schemas
  };
}

export default useApprovedSchemas;