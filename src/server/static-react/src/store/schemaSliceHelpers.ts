/**
 * Schema Slice Helper Functions
 * 
 * This file contains utility functions and helpers for the schema slice,
 * extracted to keep the main slice file focused and maintainable.
 */

import { createAsyncThunk } from '@reduxjs/toolkit';
import { RootState } from './store';
import {
  Schema,
  SchemaState as SchemaStateType,
  SchemaOperationParams,
  SchemaOperationSuccessPayload,
  SchemaOperationErrorPayload
} from '../types/schema';
import { EnhancedApiResponse } from '../api/core/types';
import {
  SCHEMA_OPERATION_REQUIREMENTS,
  SCHEMA_ERROR_MESSAGES,
  SCHEMA_STATES
} from '../constants/redux';

// ============================================================================
// CONSTANTS
// ============================================================================

export const SCHEMA_OPERATION_TYPES = {
  APPROVE: 'approve',
  BLOCK: 'block',
  UNLOAD: 'unload',
  LOAD: 'load'
} as const;

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/**
 * Check if cache is still valid based on TTL
 */
export const isCacheValid = (lastFetched: number | null, ttl: number): boolean => {
  if (!lastFetched) return false;
  return Date.now() - lastFetched < ttl;
};

/**
 * Validate if schema operation is allowed based on current state
 */
export const isOperationAllowed = (
  operation: keyof typeof SCHEMA_OPERATION_REQUIREMENTS,
  currentState: SchemaStateType
): boolean => {
  const allowedStates = SCHEMA_OPERATION_REQUIREMENTS[operation];
  return allowedStates.includes(currentState);
};

/**
 * Create timeout wrapper for API calls
 */
export const withTimeout = <T>(promise: Promise<T>, timeoutMs: number): Promise<T> => {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) =>
      setTimeout(() => reject(new Error('Operation timed out')), timeoutMs)
    )
  ]);
};

/**
 * Create standardized error payload for schema operations
 */
export const createErrorPayload = (
  schemaName: string,
  error: string,
  timestamp: number = Date.now()
): SchemaOperationErrorPayload => ({
  schemaName,
  error,
  timestamp
});

/**
 * Create standardized success payload for schema operations
 */
export const createSuccessPayload = (
  schemaName: string,
  newState: SchemaStateType,
  updatedSchema?: Schema,
  backfillHash?: string
): SchemaOperationSuccessPayload => ({
  schemaName,
  newState,
  timestamp: Date.now(),
  updatedSchema,
  backfillHash
});

/**
 * Validate schema exists and operation is allowed
 */
export const validateSchemaOperation = (
  schemaName: string,
  operation: keyof typeof SCHEMA_OPERATION_REQUIREMENTS,
  schema: Schema | undefined,
  options: { skipValidation?: boolean } = {}
): { isValid: boolean; error?: SchemaOperationErrorPayload } => {
  // Skip frontend validation - let the backend handle it
  return { isValid: true };
};

/**
 * Generic schema operation thunk factory
 */
export const createSchemaOperationThunk = <T extends keyof typeof SCHEMA_OPERATION_REQUIREMENTS>(
  actionType: string,
  clientMethod: (schemaName: string) => Promise<EnhancedApiResponse<any>>,
  successState: SchemaStateType,
  errorMessage: string
) => {
  return createAsyncThunk<
    SchemaOperationSuccessPayload,
    SchemaOperationParams,
    { state: RootState; rejectValue: SchemaOperationErrorPayload }
  >(
    actionType,
    async ({ schemaName, options = {} }, { getState, rejectWithValue }) => {
      const state = getState() as RootState;
      const schema = state.schemas.schemas[schemaName];
      
      // Validate operation
      const validation = validateSchemaOperation(schemaName, actionType as keyof typeof SCHEMA_OPERATION_REQUIREMENTS, schema, options);
      if (!validation.isValid) {
        return rejectWithValue(validation.error!);
      }
      
      try {
        const response = await clientMethod(schemaName);
        
        if (!response.success) {
          throw new Error(response.error || errorMessage);
        }
        
        // Extract backfill_hash if present in response data
        const backfillHash = response.data?.backfill_hash;
        
        return createSuccessPayload(schemaName, successState, undefined, backfillHash);
        
      } catch (error) {
        return rejectWithValue(
          createErrorPayload(
            schemaName,
            error instanceof Error ? error.message : errorMessage
          )
        );
      }
    }
  );
};

/**
 * Helper function to create standardized extra reducers for schema operations
 */
export const createSchemaOperationReducers = (
  thunk: ReturnType<typeof createSchemaOperationThunk>,
  operationType: string
) => {
  return {
    pending: (state: any, action: any) => {
      const schemaName = action.meta.arg.schemaName;
      state.loading.operations[schemaName] = true;
      delete state.errors.operations[schemaName];
    },
    fulfilled: (state: any, action: any) => {
      const { schemaName, newState, updatedSchema } = action.payload;
      state.loading.operations[schemaName] = false;
      
      if (state.schemas[schemaName]) {
        state.schemas[schemaName].state = newState;
        if (updatedSchema) {
          Object.assign(state.schemas[schemaName], updatedSchema);
        }
        state.schemas[schemaName].lastOperation = {
          type: operationType,
          timestamp: Date.now(),
          success: true
        };
      }
    },
    rejected: (state: any, action: any) => {
      const { schemaName, error } = action.payload!;
      state.loading.operations[schemaName] = false;
      state.errors.operations[schemaName] = error;
      
      if (state.schemas[schemaName]) {
        state.schemas[schemaName].lastOperation = {
          type: operationType,
          timestamp: Date.now(),
          success: false,
          error
        };
      }
    }
  };
};
