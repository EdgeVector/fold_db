/**
 * Redux Schema Slice - TASK-003: State Management Consolidation
 * 
 * This slice manages all schema-related state in a centralized manner,
 * replacing local state management in components and eliminating prop drilling.
 * Implements SCHEMA-002 compliance at the store level.
 */

import { createSlice, createAsyncThunk, createSelector, PayloadAction } from '@reduxjs/toolkit';
import { RootState } from './store';
import {
  ReduxSchemaState,
  Schema,
  SchemaState as SchemaStateType,
  FetchSchemasParams,
  FetchSchemasSuccessPayload,
  SchemaOperationParams,
  SchemaOperationSuccessPayload,
  SchemaOperationErrorPayload,
  SetLoadingPayload,
  SetErrorPayload,
  SchemaApiResponse
} from '../types/schema';
import {
  DEFAULT_SCHEMA_STATE,
  SCHEMA_ACTION_TYPES,
  SCHEMA_CACHE_TTL_MS,
  SCHEMA_FETCH_RETRY_ATTEMPTS,
  SCHEMA_OPERATION_TIMEOUT_MS,
  SCHEMA_ERROR_MESSAGES,
  SCHEMA_STATES,
  SCHEMA_OPERATION_REQUIREMENTS,
  READABLE_SCHEMA_STATES
} from '../constants/redux';
import { schemaClient } from '../api/clients/schemaClient';

// ============================================================================
// INITIAL STATE
// ============================================================================

const initialState: ReduxSchemaState = {
  schemas: {},
  loading: {
    fetch: false,
    operations: {}
  },
  errors: {
    fetch: null,
    operations: {}
  },
  lastFetched: null,
  cache: {
    ttl: SCHEMA_CACHE_TTL_MS,
    version: '1.0.0',
    lastUpdated: null
  },
  activeSchema: null
};

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Check if cache is still valid based on TTL
 */
const isCacheValid = (lastFetched: number | null, ttl: number): boolean => {
  if (!lastFetched) return false;
  return Date.now() - lastFetched < ttl;
};

/**
 * Validate if schema operation is allowed based on current state
 */
const isOperationAllowed = (
  operation: keyof typeof SCHEMA_OPERATION_REQUIREMENTS,
  currentState: SchemaStateType
): boolean => {
  const allowedStates = SCHEMA_OPERATION_REQUIREMENTS[operation];
  return allowedStates.includes(currentState);
};

/**
 * Create timeout wrapper for API calls
 */
const withTimeout = <T>(promise: Promise<T>, timeoutMs: number): Promise<T> => {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('Operation timed out')), timeoutMs)
    )
  ]);
};

// ============================================================================
// ASYNC THUNKS
// ============================================================================

/**
 * Fetch all schemas from the backend API
 * Implements caching and retry logic
 */
export const fetchSchemas = createAsyncThunk<
  FetchSchemasSuccessPayload,
  FetchSchemasParams | undefined,
  { state: RootState; rejectValue: string }
>(
  SCHEMA_ACTION_TYPES.FETCH_SCHEMAS,
  async (params = {}, { getState, rejectWithValue }) => {
    const state = getState();
    const { lastFetched, cache } = state.schemas;
    
    // Check cache validity unless force refresh is requested
    if (!params.forceRefresh && isCacheValid(lastFetched, cache.ttl)) {
      // Return current schemas from cache
      const schemas = Object.values(state.schemas.schemas);
      return {
        schemas,
        timestamp: lastFetched!
      };
    }

    // Fetch with retry logic
    let lastError: Error | null = null;
    
    for (let attempt = 1; attempt <= SCHEMA_FETCH_RETRY_ATTEMPTS; attempt++) {
      try {
        // Fetch available schemas from filesystem using SchemaClient
        const availableResponse = await schemaClient.getSchemas();
        
        if (!availableResponse.success) {
          const error = new Error(`Failed to fetch available schemas: ${availableResponse.error || 'Unknown error'}`);
          throw error;
        }
        
        // Ensure availableResponse.data is an array before calling map
        let availableSchemaNames: string[] = [];
        if (Array.isArray(availableResponse.data)) {
          availableSchemaNames = availableResponse.data.map(s => (typeof s === 'string' ? s : s.name || String(s)));
        } else if (availableResponse.data && typeof availableResponse.data === 'object') {
          // Handle case where data might be an object instead of array
          availableSchemaNames = Object.keys(availableResponse.data);
        } else {
          console.warn('Available schemas response data is not in expected format:', availableResponse.data);
          availableSchemaNames = [];
        }
        
        const availableData = { data: availableSchemaNames };
        
        // Fetch persisted schema states from database using SchemaClient
        const persistedResponse = await schemaClient.getAllSchemasWithState();
        
        if (!persistedResponse.success) {
          throw new Error(`Failed to fetch persisted schemas: ${persistedResponse.error || 'Unknown error'}`);
        }
        
        console.log('📁 Available schemas:', availableData.data || []);
        console.log('🗄️ Persisted schemas:', persistedResponse.data || {});
        
        const availableSchemas = availableData.data || [];
        // persistedResponse.data is a Record<string, string>, not an array
        const persistedStates = persistedResponse.data || {};
        
        // Ensure availableSchemas is an array and create schemas with states
        if (!Array.isArray(availableSchemas)) {
          throw new Error(`Available schemas is not an array: ${typeof availableSchemas}`);
        }
        
        const schemasWithStates = availableSchemas.map((name: string) => {
          // Check if schema already exists in Redux state (preserve optimistic updates)
          const existingSchema = state.schemas.schemas[name];
          if (existingSchema) {
            console.log('🟡 fetchSchemas: Preserving existing Redux state for', name, ':', existingSchema.state);
            return {
              name,
              state: existingSchema.state,
              fields: existingSchema.fields || {}
            };
          }
          
          // Only use persisted state if no Redux state exists
          const persistedState = persistedStates[name];
          let normalizedState = SCHEMA_STATES.AVAILABLE;
          
          if (persistedState) {
            if (typeof persistedState === 'string') {
              normalizedState = persistedState.toLowerCase();
            } else if (typeof persistedState === 'object' && persistedState.state) {
              // Handle object format like { state: 'approved' }
              normalizedState = String(persistedState.state).toLowerCase();
            } else {
              normalizedState = String(persistedState).toLowerCase();
            }
          }
          
          return {
            name,
            state: normalizedState,
            fields: {} // Will be populated below
          };
        });
        
        console.log('📋 Merged schemas for UI:', schemasWithStates);
        
        // Fetch detailed schema information for all schemas using SchemaClient
        const schemasWithDetails = await Promise.all(
          schemasWithStates.map(async (schema: any) => {
            try {
              const schemaResponse = await schemaClient.getSchema(schema.name);
              
              if (schemaResponse.success && schemaResponse.data) {
                const schemaData = schemaResponse.data;
                return {
                  ...schema,
                  ...schemaData, // Include the full schema data including schema_type
                  fields: schemaData.fields || {},
                  // Add range info if this is a range schema (using any to access schema_type)
                  rangeInfo: {
                    isRangeSchema: (schemaData as any).schema_type === 'Range',
                    rangeField: (schemaData as any).schema_type === 'Range' ? {
                      name: 'range_key',
                      type: 'Range'
                    } : undefined
                  }
                };
              } else {
                console.log(`⚠️ Schema ${schema.name} not loaded in memory, keeping basic info`);
              }
            } catch (err) {
              console.log(`⚠️ Failed to fetch details for schema ${schema.name}:`, err instanceof Error ? err.message : 'Unknown error');
            }
            return schema; // Return original if fetch fails
          })
        );
        
        console.log('✅ Final schemas for UI:', schemasWithDetails);
        
        const timestamp = Date.now();
        
        return {
          schemas: schemasWithDetails as Schema[],
          timestamp
        };
        
      } catch (error) {
        lastError = error instanceof Error ? error : new Error('Unknown error');
        
        // If this isn't the last attempt, wait before retrying
        if (attempt < SCHEMA_FETCH_RETRY_ATTEMPTS) {
          // Use shorter delays in test environment
          const isTestEnv = typeof window !== 'undefined' && (window as any).__TEST_ENV__ === true;
          const retryDelay = isTestEnv ? 10 : (1000 * attempt);
          await new Promise(resolve => setTimeout(resolve, retryDelay));
        }
      }
    }
    
    // All attempts failed - include retry count in error message
    const retryErrorMessage = `Failed to fetch schemas after ${SCHEMA_FETCH_RETRY_ATTEMPTS} attempts: ${lastError?.message || 'Unknown error'}`;
    return rejectWithValue(retryErrorMessage);
  }
);

/**
 * Approve a schema (change state from available to approved)
 */
export const approveSchema = createAsyncThunk<
  SchemaOperationSuccessPayload,
  SchemaOperationParams,
  { state: RootState; rejectValue: SchemaOperationErrorPayload }
>(
  SCHEMA_ACTION_TYPES.APPROVE_SCHEMA,
  async ({ schemaName, options = {} }, { getState, rejectWithValue }) => {
    console.log('🔵 Redux: approveSchema thunk called with:', { schemaName, options });
    const state = getState();
    const schema = state.schemas.schemas[schemaName];
    console.log('🔵 Redux: Current schema from state:', schema);
    
    if (!schema) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.SCHEMA_NOT_FOUND,
        timestamp: Date.now()
      });
    }
    
    // Validate operation is allowed
    if (!options.skipValidation && 
        !isOperationAllowed(SCHEMA_ACTION_TYPES.APPROVE_SCHEMA, schema.state)) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.INVALID_SCHEMA_STATE,
        timestamp: Date.now()
      });
    }
    
    try {
      console.log('🔵 Redux: Calling schemaClient.approveSchema for:', schemaName);
      const response = await schemaClient.approveSchema(schemaName);
      console.log('🔵 Redux: API response:', response);
      
      if (!response.success) {
        console.log('🔴 Redux: API call failed:', response.error);
        throw new Error(response.error || SCHEMA_ERROR_MESSAGES.APPROVE_FAILED);
      }
      
      const payload = {
        schemaName,
        newState: SCHEMA_STATES.APPROVED as SchemaStateType,
        timestamp: Date.now(),
        updatedSchema: undefined
      };
      console.log('🔵 Redux: Returning payload:', payload);
      return payload;
      
    } catch (error) {
      return rejectWithValue({
        schemaName,
        error: error instanceof Error ? error.message : SCHEMA_ERROR_MESSAGES.APPROVE_FAILED,
        timestamp: Date.now()
      });
    }
  }
);

/**
 * Block a schema (change state to blocked)
 */
export const blockSchema = createAsyncThunk<
  SchemaOperationSuccessPayload,
  SchemaOperationParams,
  { state: RootState; rejectValue: SchemaOperationErrorPayload }
>(
  SCHEMA_ACTION_TYPES.BLOCK_SCHEMA,
  async ({ schemaName, options = {} }, { getState, rejectWithValue }) => {
    const state = getState();
    const schema = state.schemas.schemas[schemaName];
    
    if (!schema) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.SCHEMA_NOT_FOUND,
        timestamp: Date.now()
      });
    }
    
    // Validate operation is allowed
    if (!options.skipValidation && 
        !isOperationAllowed(SCHEMA_ACTION_TYPES.BLOCK_SCHEMA, schema.state)) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.INVALID_SCHEMA_STATE,
        timestamp: Date.now()
      });
    }
    
    try {
      const response = await schemaClient.blockSchema(schemaName);
      
      if (!response.success) {
        throw new Error(response.error || SCHEMA_ERROR_MESSAGES.BLOCK_FAILED);
      }
      
      return {
        schemaName,
        newState: SCHEMA_STATES.BLOCKED as SchemaStateType,
        timestamp: Date.now(),
        updatedSchema: undefined
      };
      
    } catch (error) {
      return rejectWithValue({
        schemaName,
        error: error instanceof Error ? error.message : SCHEMA_ERROR_MESSAGES.BLOCK_FAILED,
        timestamp: Date.now()
      });
    }
  }
);

/**
 * Unload a schema (remove from active use)
 */
export const unloadSchema = createAsyncThunk<
  SchemaOperationSuccessPayload,
  SchemaOperationParams,
  { state: RootState; rejectValue: SchemaOperationErrorPayload }
>(
  SCHEMA_ACTION_TYPES.UNLOAD_SCHEMA,
  async ({ schemaName, options = {} }, { getState, rejectWithValue }) => {
    const state = getState();
    const schema = state.schemas.schemas[schemaName];
    
    if (!schema) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.SCHEMA_NOT_FOUND,
        timestamp: Date.now()
      });
    }
    
    // Validate operation is allowed
    if (!options.skipValidation && 
        !isOperationAllowed(SCHEMA_ACTION_TYPES.UNLOAD_SCHEMA, schema.state)) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.INVALID_SCHEMA_STATE,
        timestamp: Date.now()
      });
    }
    
    try {
      const response = await schemaClient.unloadSchema(schemaName);
      
      if (!response.success) {
        throw new Error(response.error || SCHEMA_ERROR_MESSAGES.UNLOAD_FAILED);
      }
      
      return {
        schemaName,
        newState: SCHEMA_STATES.AVAILABLE as SchemaStateType,
        timestamp: Date.now(),
        updatedSchema: undefined
      };
      
    } catch (error) {
      return rejectWithValue({
        schemaName,
        error: error instanceof Error ? error.message : SCHEMA_ERROR_MESSAGES.UNLOAD_FAILED,
        timestamp: Date.now()
      });
    }
  }
);

/**
 * Load a schema (change state from available to approved)
 */
export const loadSchema = createAsyncThunk<
  SchemaOperationSuccessPayload,
  SchemaOperationParams,
  { state: RootState; rejectValue: SchemaOperationErrorPayload }
>(
  SCHEMA_ACTION_TYPES.LOAD_SCHEMA,
  async ({ schemaName, options = {} }, { getState, rejectWithValue }) => {
    const state = getState();
    const schema = state.schemas.schemas[schemaName];
    
    if (!schema) {
      return rejectWithValue({
        schemaName,
        error: SCHEMA_ERROR_MESSAGES.SCHEMA_NOT_FOUND,
        timestamp: Date.now()
      });
    }
    
    try {
      const response = await schemaClient.loadSchema(schemaName);
      
      if (!response.success) {
        throw new Error(response.error || SCHEMA_ERROR_MESSAGES.LOAD_FAILED);
      }
      
      return {
        schemaName,
        newState: SCHEMA_STATES.APPROVED as SchemaStateType,
        timestamp: Date.now(),
        updatedSchema: undefined
      };
      
    } catch (error) {
      return rejectWithValue({
        schemaName,
        error: error instanceof Error ? error.message : SCHEMA_ERROR_MESSAGES.LOAD_FAILED,
        timestamp: Date.now()
      });
    }
  }
);

// ============================================================================
// SCHEMA SLICE
// ============================================================================

const schemaSlice = createSlice({
  name: 'schemas',
  initialState,
  reducers: {
    /**
     * Set the currently active schema
     */
    setActiveSchema: (state, action: PayloadAction<string | null>) => {
      state.activeSchema = action.payload;
    },
    
    /**
     * Update a specific schema's status
     */
    updateSchemaStatus: (state, action: PayloadAction<{ schemaName: string; newState: SchemaStateType }>) => {
      const { schemaName, newState } = action.payload;
      if (state.schemas[schemaName]) {
        state.schemas[schemaName].state = newState;
        state.schemas[schemaName].lastOperation = {
          type: 'approve',
          timestamp: Date.now(),
          success: true
        };
      }
    },
    
    /**
     * Set loading state for operations
     */
    setLoading: (state, action: PayloadAction<SetLoadingPayload>) => {
      const { operation, isLoading, schemaName } = action.payload;
      
      if (operation === 'fetch') {
        state.loading.fetch = isLoading;
      } else if (schemaName) {
        state.loading.operations[schemaName] = isLoading;
      }
    },
    
    /**
     * Set error state for operations
     */
    setError: (state, action: PayloadAction<SetErrorPayload>) => {
      const { operation, error, schemaName } = action.payload;
      
      if (operation === 'fetch') {
        state.errors.fetch = error;
      } else if (schemaName) {
        state.errors.operations[schemaName] = error || '';
      }
    },
    
    /**
     * Clear all errors
     */
    clearError: (state) => {
      state.errors.fetch = null;
      state.errors.operations = {};
    },
    
    /**
     * Clear error for specific operation
     */
    clearOperationError: (state, action: PayloadAction<string>) => {
      const schemaName = action.payload;
      delete state.errors.operations[schemaName];
    },
    
    /**
     * Invalidate cache to force next fetch
     */
    invalidateCache: (state) => {
      state.lastFetched = null;
      state.cache.lastUpdated = null;
    },
    
    /**
     * Reset all schema state
     */
    resetSchemas: (state) => {
      Object.assign(state, initialState);
    }
  },
  extraReducers: (builder) => {
    builder
      // fetchSchemas cases
      .addCase(fetchSchemas.pending, (state) => {
        state.loading.fetch = true;
        state.errors.fetch = null;
      })
      .addCase(fetchSchemas.fulfilled, (state, action) => {
        state.loading.fetch = false;
        state.errors.fetch = null;
        
        // Update schemas
        const schemasMap: Record<string, Schema> = {};
        action.payload.schemas.forEach(schema => {
          schemasMap[schema.name] = schema;
        });
        state.schemas = schemasMap;
        
        // Update cache info
        state.lastFetched = action.payload.timestamp;
        state.cache.lastUpdated = action.payload.timestamp;
      })
      .addCase(fetchSchemas.rejected, (state, action) => {
        state.loading.fetch = false;
        state.errors.fetch = action.payload || SCHEMA_ERROR_MESSAGES.FETCH_FAILED;
      })
      
      // approveSchema cases
      .addCase(approveSchema.pending, (state, action) => {
        const schemaName = action.meta.arg.schemaName;
        state.loading.operations[schemaName] = true;
        delete state.errors.operations[schemaName];
      })
      .addCase(approveSchema.fulfilled, (state, action) => {
        const { schemaName, newState, updatedSchema } = action.payload;
        console.log('🔵 Redux: approveSchema.fulfilled', { schemaName, newState, updatedSchema });
        state.loading.operations[schemaName] = false;
        
        if (state.schemas[schemaName]) {
          console.log('🔵 Redux: Updating schema state from', state.schemas[schemaName].state, 'to', newState);
          state.schemas[schemaName].state = newState;
          if (updatedSchema) {
            Object.assign(state.schemas[schemaName], updatedSchema);
          }
          state.schemas[schemaName].lastOperation = {
            type: 'approve',
            timestamp: Date.now(),
            success: true
          };
          console.log('🔵 Redux: Schema state updated to', state.schemas[schemaName].state);
        } else {
          console.log('🔴 Redux: Schema not found in state:', schemaName);
        }
      })
      .addCase(approveSchema.rejected, (state, action) => {
        const { schemaName, error } = action.payload!;
        state.loading.operations[schemaName] = false;
        state.errors.operations[schemaName] = error;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].lastOperation = {
            type: 'approve',
            timestamp: Date.now(),
            success: false,
            error
          };
        }
      })
      
      // blockSchema cases
      .addCase(blockSchema.pending, (state, action) => {
        const schemaName = action.meta.arg.schemaName;
        state.loading.operations[schemaName] = true;
        delete state.errors.operations[schemaName];
      })
      .addCase(blockSchema.fulfilled, (state, action) => {
        const { schemaName, newState, updatedSchema } = action.payload;
        state.loading.operations[schemaName] = false;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].state = newState;
          if (updatedSchema) {
            Object.assign(state.schemas[schemaName], updatedSchema);
          }
          state.schemas[schemaName].lastOperation = {
            type: 'block',
            timestamp: Date.now(),
            success: true
          };
        }
      })
      .addCase(blockSchema.rejected, (state, action) => {
        const { schemaName, error } = action.payload!;
        state.loading.operations[schemaName] = false;
        state.errors.operations[schemaName] = error;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].lastOperation = {
            type: 'block',
            timestamp: Date.now(),
            success: false,
            error
          };
        }
      })
      
      // unloadSchema cases
      .addCase(unloadSchema.pending, (state, action) => {
        const schemaName = action.meta.arg.schemaName;
        state.loading.operations[schemaName] = true;
        delete state.errors.operations[schemaName];
      })
      .addCase(unloadSchema.fulfilled, (state, action) => {
        const { schemaName, newState, updatedSchema } = action.payload;
        state.loading.operations[schemaName] = false;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].state = newState;
          if (updatedSchema) {
            Object.assign(state.schemas[schemaName], updatedSchema);
          }
          state.schemas[schemaName].lastOperation = {
            type: 'unload',
            timestamp: Date.now(),
            success: true
          };
        }
      })
      .addCase(unloadSchema.rejected, (state, action) => {
        const { schemaName, error } = action.payload!;
        state.loading.operations[schemaName] = false;
        state.errors.operations[schemaName] = error;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].lastOperation = {
            type: 'unload',
            timestamp: Date.now(),
            success: false,
            error
          };
        }
      })
      
      // loadSchema cases
      .addCase(loadSchema.pending, (state, action) => {
        const schemaName = action.meta.arg.schemaName;
        state.loading.operations[schemaName] = true;
        delete state.errors.operations[schemaName];
      })
      .addCase(loadSchema.fulfilled, (state, action) => {
        const { schemaName, newState, updatedSchema } = action.payload;
        state.loading.operations[schemaName] = false;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].state = newState;
          if (updatedSchema) {
            Object.assign(state.schemas[schemaName], updatedSchema);
          }
          state.schemas[schemaName].lastOperation = {
            type: 'load',
            timestamp: Date.now(),
            success: true
          };
        }
      })
      .addCase(loadSchema.rejected, (state, action) => {
        const { schemaName, error } = action.payload!;
        state.loading.operations[schemaName] = false;
        state.errors.operations[schemaName] = error;
        
        if (state.schemas[schemaName]) {
          state.schemas[schemaName].lastOperation = {
            type: 'load',
            timestamp: Date.now(),
            success: false,
            error
          };
        }
      });
  }
});

// ============================================================================
// SELECTORS (SCHEMA-002 COMPLIANT)
// ============================================================================

// Base selectors
export const selectSchemaState = (state: RootState) => state.schemas;
export const selectAllSchemas = (state: RootState) => Object.values(state.schemas.schemas);
export const selectSchemasById = (state: RootState) => state.schemas.schemas;

// SCHEMA-002 compliant selectors - only approved schemas for operations
export const selectApprovedSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) => schemas.filter(schema => {
    // Use the same normalization logic as the hook
    const normalizedState = typeof schema.state === 'string'
      ? schema.state.toLowerCase()
      : typeof schema.state === 'object' && schema.state !== null && (schema.state as any).state
        ? String((schema.state as any).state).toLowerCase()
        : String(schema.state || '').toLowerCase();
    return normalizedState === SCHEMA_STATES.APPROVED;
  })
);

export const selectAvailableSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) => schemas.filter(schema => schema.state === SCHEMA_STATES.AVAILABLE)
);

export const selectBlockedSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) => schemas.filter(schema => schema.state === SCHEMA_STATES.BLOCKED)
);

// Range schema selectors
export const selectApprovedRangeSchemas = createSelector(
  [selectApprovedSchemas],
  (schemas: Schema[]) => schemas.filter(schema => schema.rangeInfo?.isRangeSchema === true)
);

export const selectAvailableRangeSchemas = createSelector(
  [selectAvailableSchemas],
  (schemas: Schema[]) => schemas.filter(schema => schema.rangeInfo?.isRangeSchema === true)
);

// Loading and error selectors
export const selectSchemaLoading = (state: RootState) => state.schemas.loading;
export const selectSchemaErrors = (state: RootState) => state.schemas.errors;
export const selectFetchLoading = (state: RootState) => state.schemas.loading.fetch;
export const selectFetchError = (state: RootState) => state.schemas.errors.fetch;

// Schema-specific selectors
export const selectSchemaById = (schemaName: string) => (state: RootState) =>
  state.schemas.schemas[schemaName] || null;

export const selectSchemaOperationState = (schemaName: string) => (state: RootState) => ({
  loading: state.schemas.loading.operations[schemaName] || false,
  error: state.schemas.errors.operations[schemaName] || null
});

// Cache selectors
export const selectCacheInfo = createSelector(
  [selectSchemaState],
  (schemaState) => ({
    isValid: isCacheValid(schemaState.lastFetched, schemaState.cache.ttl),
    lastFetched: schemaState.lastFetched,
    ttl: schemaState.cache.ttl
  })
);

// Active schema selectors
export const selectActiveSchema = (state: RootState) => state.schemas.activeSchema;
export const selectActiveSchemaData = createSelector(
  [selectActiveSchema, selectSchemasById],
  (activeSchemaName, schemasById) =>
    activeSchemaName ? schemasById[activeSchemaName] || null : null
);

// Export actions and reducer
export const {
  setActiveSchema,
  updateSchemaStatus,
  setLoading,
  setError,
  clearError,
  clearOperationError,
  invalidateCache,
  resetSchemas
} = schemaSlice.actions;

export default schemaSlice.reducer;