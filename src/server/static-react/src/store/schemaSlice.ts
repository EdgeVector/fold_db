/**
 * Redux Schema Slice - TASK-003: State Management Consolidation
 *
 * This slice manages all schema-related state in a centralized manner,
 * replacing local state management in components and eliminating prop drilling.
 * Implements SCHEMA-002 compliance at the store level.
 */

import {
  createSlice,
  createAsyncThunk,
  createSelector,
  PayloadAction,
} from "@reduxjs/toolkit";
import { RootState } from "./store";
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
  SchemaApiResponse,
} from "../types/schema";
import {
  DEFAULT_SCHEMA_STATE,
  SCHEMA_ACTION_TYPES,
  SCHEMA_CACHE_TTL_MS,
  SCHEMA_FETCH_RETRY_ATTEMPTS,
  SCHEMA_OPERATION_TIMEOUT_MS,
  SCHEMA_ERROR_MESSAGES,
  SCHEMA_STATES,
  SCHEMA_OPERATION_REQUIREMENTS,
  READABLE_SCHEMA_STATES,
} from "../constants/redux";
import { schemaClient as sharedSchemaClient } from "../api/clients/schemaClient";
import {
  SCHEMA_OPERATION_TYPES,
  isCacheValid,
  createSchemaOperationThunk,
  createSchemaOperationReducers,
} from "./schemaSliceHelpers";

// ============================================================================
// INITIAL STATE
// ============================================================================

const initialState: ReduxSchemaState = {
  schemas: {},
  loading: {
    fetch: false,
    operations: {},
  },
  errors: {
    fetch: null,
    operations: {},
  },
  lastFetched: null,
  cache: {
    ttl: SCHEMA_CACHE_TTL_MS,
    version: "1.0.0",
    lastUpdated: null,
  },
  activeSchema: null,
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
        timestamp: lastFetched!,
      };
    }

    // Clear API client cache when force refresh is requested
    if (params.forceRefresh) {
      sharedSchemaClient.clearCache();
    }

    // Fetch with retry logic
    let lastError: Error | null = null;

    for (let attempt = 1; attempt <= SCHEMA_FETCH_RETRY_ATTEMPTS; attempt++) {
      try {
        // Fetch schemas with their states from the backend
        const availableResponse = await sharedSchemaClient.getSchemas();

        if (!availableResponse.success) {
          const error = new Error(
            `Failed to fetch schemas: ${availableResponse.error || "Unknown error"}`,
          );
          throw error;
        }

        // The backend returns full SchemaWithState objects - use them directly
        const rawSchemas = availableResponse.data || [];

        if (!Array.isArray(rawSchemas)) {
          throw new Error(
            `Schemas response is not an array: ${typeof rawSchemas}`,
          );
        }

        // Use the backend schema objects directly without transformation
        // Only normalize the state field to lowercase for consistency
        const schemas = rawSchemas
          .map((schema: Record<string, unknown>) => {
            // Ensure schema has a name field - this is critical for display
            if (!schema.name) {
              console.warn("Schema missing name field:", schema);
              // Try to extract name from nested structure if present
              const nested = schema.schema as Record<string, unknown> | undefined;
              if (nested && nested.name) {
                schema.name = nested.name;
              } else {
                console.error(
                  "Schema has no name field and cannot be displayed:",
                  schema,
                );
                // Skip schemas without names - they can't be displayed
                return null;
              }
            }

            // Normalize state to lowercase string if it exists
            let normalizedState = SCHEMA_STATES.AVAILABLE;

            if (schema.state) {
              if (typeof schema.state === "string") {
                normalizedState = schema.state.toLowerCase();
              } else if (
                typeof schema.state === "object" &&
                schema.state !== null &&
                (schema.state as Record<string, unknown>).state
              ) {
                // Handle object format like { state: 'approved' }
                normalizedState = String((schema.state as Record<string, unknown>).state).toLowerCase();
              } else {
                normalizedState = String(schema.state).toLowerCase();
              }
            }

            // Return the full schema object from backend with normalized state
            return {
              ...schema,
              state: normalizedState,
            };
          })
          .filter((schema: Record<string, unknown> | null) => schema !== null); // Remove any null schemas (those without names)

        const timestamp = Date.now();

        return {
          schemas: schemas as Schema[],
          timestamp,
        };
      } catch (error) {
        lastError = error instanceof Error ? error : new Error("Unknown error");

        // If this isn't the last attempt, wait before retrying
        if (attempt < SCHEMA_FETCH_RETRY_ATTEMPTS) {
          // Use shorter delays in test environment
          const isTestEnv =
            typeof window !== "undefined" &&
            (window as unknown as Record<string, unknown>).__TEST_ENV__ === true;
          const retryDelay = isTestEnv ? 10 : 1000 * attempt;
          await new Promise((resolve) => setTimeout(resolve, retryDelay));
        }
      }
    }

    // All attempts failed - include retry count in error message
    const retryErrorMessage = `Failed to fetch schemas after ${SCHEMA_FETCH_RETRY_ATTEMPTS} attempts: ${lastError?.message || "Unknown error"}`;
    return rejectWithValue(retryErrorMessage);
  },
);

/**
 * Schema operation thunks using the factory function
 */
export const approveSchema = createSchemaOperationThunk(
  SCHEMA_ACTION_TYPES.APPROVE_SCHEMA,
  (name: string) => sharedSchemaClient.approveSchema(name),
  SCHEMA_STATES.APPROVED as SchemaStateType,
  SCHEMA_ERROR_MESSAGES.APPROVE_FAILED,
);

export const blockSchema = createSchemaOperationThunk(
  SCHEMA_ACTION_TYPES.BLOCK_SCHEMA,
  (name: string) => sharedSchemaClient.blockSchema(name),
  SCHEMA_STATES.BLOCKED as SchemaStateType,
  SCHEMA_ERROR_MESSAGES.BLOCK_FAILED,
);

export const unloadSchema = createSchemaOperationThunk(
  SCHEMA_ACTION_TYPES.UNLOAD_SCHEMA,
  (name: string) => sharedSchemaClient.unloadSchema(name),
  SCHEMA_STATES.AVAILABLE as SchemaStateType,
  SCHEMA_ERROR_MESSAGES.UNLOAD_FAILED,
);

export const loadSchema = createSchemaOperationThunk(
  SCHEMA_ACTION_TYPES.LOAD_SCHEMA,
  (name: string) => sharedSchemaClient.loadSchema(name),
  SCHEMA_STATES.APPROVED as SchemaStateType,
  SCHEMA_ERROR_MESSAGES.LOAD_FAILED,
);

// ============================================================================
// SCHEMA SLICE
// ============================================================================

const schemaSlice = createSlice({
  name: "schemas",
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
    updateSchemaStatus: (
      state,
      action: PayloadAction<{ schemaName: string; newState: SchemaStateType }>,
    ) => {
      const { schemaName, newState } = action.payload;
      if (state.schemas[schemaName]) {
        state.schemas[schemaName].state = newState;
        state.schemas[schemaName].lastOperation = {
          type: SCHEMA_OPERATION_TYPES.APPROVE,
          timestamp: Date.now(),
          success: true,
        };
      }
    },

    /**
     * Set loading state for operations
     */
    setLoading: (state, action: PayloadAction<SetLoadingPayload>) => {
      const { operation, isLoading, schemaName } = action.payload;

      if (operation === "fetch") {
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

      if (operation === "fetch") {
        state.errors.fetch = error;
      } else if (schemaName) {
        state.errors.operations[schemaName] = error || "";
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
    },
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
        action.payload.schemas.forEach((schema) => {
          schemasMap[schema.name] = schema;
        });
        state.schemas = schemasMap;

        // Update cache info
        state.lastFetched = action.payload.timestamp;
        state.cache.lastUpdated = action.payload.timestamp;
      })
      .addCase(fetchSchemas.rejected, (state, action) => {
        state.loading.fetch = false;
        state.errors.fetch =
          action.payload || SCHEMA_ERROR_MESSAGES.FETCH_FAILED;
      })

      // Schema operation cases using helper function
      .addCase(
        approveSchema.pending,
        createSchemaOperationReducers(
          approveSchema,
          SCHEMA_OPERATION_TYPES.APPROVE,
        ).pending,
      )
      .addCase(
        approveSchema.fulfilled,
        createSchemaOperationReducers(
          approveSchema,
          SCHEMA_OPERATION_TYPES.APPROVE,
        ).fulfilled,
      )
      .addCase(
        approveSchema.rejected,
        createSchemaOperationReducers(
          approveSchema,
          SCHEMA_OPERATION_TYPES.APPROVE,
        ).rejected,
      )

      .addCase(
        blockSchema.pending,
        createSchemaOperationReducers(blockSchema, SCHEMA_OPERATION_TYPES.BLOCK)
          .pending,
      )
      .addCase(
        blockSchema.fulfilled,
        createSchemaOperationReducers(blockSchema, SCHEMA_OPERATION_TYPES.BLOCK)
          .fulfilled,
      )
      .addCase(
        blockSchema.rejected,
        createSchemaOperationReducers(blockSchema, SCHEMA_OPERATION_TYPES.BLOCK)
          .rejected,
      )

      .addCase(
        unloadSchema.pending,
        createSchemaOperationReducers(
          unloadSchema,
          SCHEMA_OPERATION_TYPES.UNLOAD,
        ).pending,
      )
      .addCase(
        unloadSchema.fulfilled,
        createSchemaOperationReducers(
          unloadSchema,
          SCHEMA_OPERATION_TYPES.UNLOAD,
        ).fulfilled,
      )
      .addCase(
        unloadSchema.rejected,
        createSchemaOperationReducers(
          unloadSchema,
          SCHEMA_OPERATION_TYPES.UNLOAD,
        ).rejected,
      )

      .addCase(
        loadSchema.pending,
        createSchemaOperationReducers(loadSchema, SCHEMA_OPERATION_TYPES.LOAD)
          .pending,
      )
      .addCase(
        loadSchema.fulfilled,
        createSchemaOperationReducers(loadSchema, SCHEMA_OPERATION_TYPES.LOAD)
          .fulfilled,
      )
      .addCase(
        loadSchema.rejected,
        createSchemaOperationReducers(loadSchema, SCHEMA_OPERATION_TYPES.LOAD)
          .rejected,
      );
  },
});

// ============================================================================
// SELECTORS (SCHEMA-002 COMPLIANT)
// ============================================================================

// Base selectors
export const selectSchemaState = (state: RootState) => state.schemas;
export const selectSchemasById = (state: RootState) => state.schemas.schemas;

// Memoized selector to avoid creating new array on every call
export const selectAllSchemas = createSelector([selectSchemasById], (schemas) =>
  Object.values(schemas),
);

// SCHEMA-002 compliant selectors - only approved schemas for operations
export const selectApprovedSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) =>
    schemas.filter((schema) => {
      // Use the same normalization logic as the hook
      const normalizedState =
        typeof schema.state === "string"
          ? schema.state.toLowerCase()
          : typeof schema.state === "object" &&
              schema.state !== null &&
              (schema.state as unknown as Record<string, unknown>).state
            ? String((schema.state as unknown as Record<string, unknown>).state).toLowerCase()
            : String(schema.state || "").toLowerCase();
      return normalizedState === SCHEMA_STATES.APPROVED;
    }),
);

export const selectAvailableSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) =>
    schemas.filter((schema) => schema.state === SCHEMA_STATES.AVAILABLE),
);

export const selectBlockedSchemas = createSelector(
  [selectAllSchemas],
  (schemas: Schema[]) =>
    schemas.filter((schema) => schema.state === SCHEMA_STATES.BLOCKED),
);

// Range schema selectors
export const selectApprovedRangeSchemas = createSelector(
  [selectApprovedSchemas],
  (schemas: Schema[]) =>
    schemas.filter((schema) => schema.rangeInfo?.isRangeSchema === true),
);

export const selectAvailableRangeSchemas = createSelector(
  [selectAvailableSchemas],
  (schemas: Schema[]) =>
    schemas.filter((schema) => schema.rangeInfo?.isRangeSchema === true),
);

// Loading and error selectors
export const selectSchemaLoading = (state: RootState) => state.schemas.loading;
export const selectSchemaErrors = (state: RootState) => state.schemas.errors;
export const selectFetchLoading = (state: RootState) =>
  state.schemas.loading.fetch;
export const selectFetchError = (state: RootState) =>
  state.schemas.errors.fetch;

// Schema-specific selectors
export const selectSchemaById = (schemaName: string) => (state: RootState) =>
  state.schemas.schemas[schemaName] || null;

export const selectSchemaOperationState =
  (schemaName: string) => (state: RootState) => ({
    loading: state.schemas.loading.operations[schemaName] || false,
    error: state.schemas.errors.operations[schemaName] || null,
  });

// Cache selectors
export const selectCacheInfo = createSelector(
  [selectSchemaState],
  (schemaState) => ({
    isValid: isCacheValid(schemaState.lastFetched, schemaState.cache.ttl),
    lastFetched: schemaState.lastFetched,
    ttl: schemaState.cache.ttl,
  }),
);

// Active schema selectors
export const selectActiveSchema = (state: RootState) =>
  state.schemas.activeSchema;
export const selectActiveSchemaData = createSelector(
  [selectActiveSchema, selectSchemasById],
  (activeSchemaName, schemasById) =>
    activeSchemaName ? schemasById[activeSchemaName] || null : null,
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
  resetSchemas,
} = schemaSlice.actions;

export default schemaSlice.reducer;
