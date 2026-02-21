/**
 * TypeScript Type Definitions for Schema State Management
 * TASK-003: State Management Consolidation with Redux
 * 
 * This file contains all TypeScript interfaces and types for the schema Redux slice.
 * Ensures type safety and proper intellisense support for schema state management.
 * 
 * Note: We compose UI state types with Rust-generated domain types from ts-rs.
 */

// Import auto-generated types from Rust backend via ts-rs
// Using @generated alias that points to bindings/ directory where ts-rs writes them
import type { 
  DeclarativeSchemaDefinition,
} from '@generated/generated';

// Re-export backend type as BackendSchema for backward compatibility
export type BackendSchema = DeclarativeSchemaDefinition;

// ============================================================================
// CORE SCHEMA TYPES
// ============================================================================

/**
 * Valid schema states as per SCHEMA-002 compliance
 */
export type SchemaState = 'available' | 'approved' | 'blocked' | 'loading' | 'error';

/**
 * Schema field type definitions
 */
export interface SchemaField {
  /** Field name */
  name: string;
  
  /** Field data type (string, number, boolean, etc.) */
  type: string;
  
  /** Whether the field is required */
  required?: boolean;
  
  /** Field description */
  description?: string;
  
  /** Default value for the field */
  defaultValue?: unknown;

  /** Validation constraints */
  constraints?: Record<string, unknown>;
}

/**
 * Range schema specific properties
 */
export interface RangeSchemaInfo {
  /** Whether this is a range schema */
  isRangeSchema: boolean;
  
  /** Range field information if applicable */
  rangeField?: {
    name: string;
    type: string;
    minValue?: number;
    maxValue?: number;
  };
}

/**
 * Individual schema structure as returned from the backend API
 * This is the backend DeclarativeSchemaDefinition with SchemaState (SchemaWithState in Rust)
 */
export type Schema = BackendSchema & {
  /** Current state of the schema (available, approved, blocked, etc.) */
  state: SchemaState;

  /** Optional high-level definition/structure metadata (UI-only field) */
  definition?: Record<string, unknown>;

  /** Schema metadata (UI-only field) */
  metadata?: {
    /** Creation timestamp */
    createdAt?: string;

    /** Last updated timestamp */
    updatedAt?: string;

    /** Schema version */
    version?: string;

    /** Schema description */
    description?: string;

    /** Schema tags */
    tags?: string[];
  };

  /** Range schema information (UI-only field) */
  rangeInfo?: RangeSchemaInfo;

  /** Whether schema is currently being processed (UI-only field) */
  processing?: boolean;

  /** Last operation performed on this schema (UI-only field) */
  lastOperation?: {
    type: 'approve' | 'block' | 'unload' | 'load';
    timestamp: number;
    success: boolean;
    error?: string;
  };
};

// ============================================================================
// REDUX STATE TYPES
// ============================================================================

/**
 * Loading state structure for schema operations
 */
export interface SchemaLoadingState {
  /** Global fetch operation loading state */
  fetch: boolean;
  
  /** Per-schema operation loading states */
  operations: Record<string, boolean>;
}

/**
 * Error state structure for schema operations
 */
export interface SchemaErrorState {
  /** Global fetch operation error */
  fetch: string | null;
  
  /** Per-schema operation errors */
  operations: Record<string, string>;
}

/**
 * Cache configuration and status
 */
export interface SchemaCacheState {
  /** Cache time-to-live in milliseconds */
  ttl: number;
  
  /** Cache version for invalidation */
  version: string;
  
  /** Last cache update timestamp */
  lastUpdated: number | null;
}

/**
 * Complete Redux schema state structure
 */
export interface ReduxSchemaState {
  /** All schemas keyed by schema name */
  schemas: Record<string, Schema>;
  
  /** Loading states for various operations */
  loading: SchemaLoadingState;
  
  /** Error states for various operations */
  errors: SchemaErrorState;
  
  /** Timestamp of last successful fetch */
  lastFetched: number | null;
  
  /** Cache configuration and status */
  cache: SchemaCacheState;
  
  /** Currently active/selected schema */
  activeSchema: string | null;
}

// ============================================================================
// ACTION PAYLOAD TYPES
// ============================================================================

/**
 * Payload for schema fetch success action
 */
export interface FetchSchemasSuccessPayload {
  /** Array of schemas from the API */
  schemas: Schema[];
  
  /** Timestamp when schemas were fetched */
  timestamp: number;
}

/**
 * Payload for schema operation actions (approve, block, unload)
 */
export interface SchemaOperationPayload {
  /** Schema name to operate on */
  schemaName: string;

  /** Additional operation parameters */
  params?: Record<string, unknown>;
}

/**
 * Payload for schema operation success
 */
export interface SchemaOperationSuccessPayload {
  /** Schema name that was operated on */
  schemaName: string;
  
  /** New schema state after operation */
  newState: SchemaState;
  
  /** Operation timestamp */
  timestamp: number;
  
  /** Updated schema data */
  updatedSchema?: Partial<Schema>;
  
  /** Unique backfill hash for tracking (when applicable) */
  backfillHash?: string;
}

/**
 * Payload for schema operation failure
 */
export interface SchemaOperationErrorPayload {
  /** Schema name that failed */
  schemaName: string;
  
  /** Error message */
  error: string;
  
  /** Error timestamp */
  timestamp: number;
}

/**
 * Payload for setting loading states
 */
export interface SetLoadingPayload {
  /** Operation type (fetch or specific schema operation) */
  operation: 'fetch' | string;
  
  /** Whether operation is loading */
  isLoading: boolean;
  
  /** Optional schema name for schema-specific operations */
  schemaName?: string;
}

/**
 * Payload for setting error states
 */
export interface SetErrorPayload {
  /** Operation type (fetch or specific schema operation) */
  operation: 'fetch' | string;
  
  /** Error message */
  error: string | null;
  
  /** Optional schema name for schema-specific operations */
  schemaName?: string;
}

// ============================================================================
// ASYNC THUNK TYPES
// ============================================================================

/**
 * Parameters for fetchSchemas async thunk
 */
export interface FetchSchemasParams {
  /** Force refresh even if cache is valid */
  forceRefresh?: boolean;
  
  /** Include additional metadata in response */
  includeMetadata?: boolean;
}

/**
 * Parameters for schema operation async thunks
 */
export interface SchemaOperationParams {
  /** Schema name to operate on */
  schemaName: string;
  
  /** Additional operation-specific parameters */
  options?: {
    /** Skip validation checks */
    skipValidation?: boolean;
    
    /** Custom timeout for this operation */
    timeout?: number;
    
    /** Additional metadata to include */
    metadata?: Record<string, unknown>;
  };
}

// ============================================================================
// SELECTOR TYPES
// ============================================================================

/**
 * Return type for schema selectors
 */
export interface SchemaSelectorsReturn {
  /** All schemas */
  allSchemas: Schema[];
  
  /** Only approved schemas (SCHEMA-002 compliant) */
  approvedSchemas: Schema[];
  
  /** Only available schemas */
  availableSchemas: Schema[];
  
  /** Only blocked schemas */
  blockedSchemas: Schema[];
  
  /** Range schemas only */
  rangeSchemas: Schema[];
  
  /** Loading states */
  isLoading: boolean;
  fetchLoading: boolean;
  operationLoading: Record<string, boolean>;
  
  /** Error states */
  errors: {
    fetch: string | null;
    operations: Record<string, string>;
  };
  
  /** Cache information */
  cacheInfo: {
    isValid: boolean;
    lastFetched: number | null;
    ttl: number;
  };
}

/**
 * Parameters for schema-specific selectors
 */
export interface SchemaSpecificSelectorParams {
  /** Schema name to select */
  schemaName: string;
}

/**
 * Return type for schema-specific selectors
 */
export interface SchemaSpecificSelectorReturn {
  /** The specific schema */
  schema: Schema | null;
  
  /** Loading state for this schema */
  isLoading: boolean;
  
  /** Error state for this schema */
  error: string | null;
  
  /** Whether schema can be operated on */
  canApprove: boolean;
  canBlock: boolean;
  canUnload: boolean;
}

// ============================================================================
// MIDDLEWARE TYPES
// ============================================================================

/**
 * Schema middleware configuration
 */
export interface SchemaMiddlewareConfig {
  /** Enable action logging */
  enableLogging: boolean;
  
  /** Enable performance tracking */
  enablePerformanceTracking: boolean;
  
  /** Performance threshold in milliseconds */
  performanceThreshold: number;
  
  /** Enable error reporting */
  enableErrorReporting: boolean;
}

/**
 * Performance tracking data
 */
export interface PerformanceTrackingData {
  /** Action type that was tracked */
  actionType: string;
  
  /** Duration in milliseconds */
  duration: number;
  
  /** Timestamp when action started */
  startTime: number;
  
  /** Timestamp when action completed */
  endTime: number;
  
  /** Whether performance threshold was exceeded */
  thresholdExceeded: boolean;
}

// ============================================================================
// API INTEGRATION TYPES
// ============================================================================

/**
 * Schema API response structure
 */
export interface SchemaApiResponse {
  /** Whether the API call was successful */
  success: boolean;
  
  /** Response data */
  data?: {
    /** Array of schemas */
    schemas?: Schema[];
    
    /** Single schema for individual operations */
    schema?: Schema;
    
    /** Additional metadata */
    metadata?: Record<string, unknown>;
  };

  /** Error information if unsuccessful */
  error?: {
    /** Error code */
    code: string;

    /** Human-readable error message */
    message: string;

    /** Additional error details */
    details?: unknown;
  };
  
  /** Response timestamp */
  timestamp: number;
}

/**
 * Schema operation API request structure
 */
export interface SchemaOperationRequest {
  /** Schema name to operate on */
  schemaName: string;
  
  /** Operation type */
  operation: 'approve' | 'block' | 'unload' | 'load';
  
  /** Additional request parameters */
  params?: Record<string, unknown>;
  
  /** Request metadata */
  metadata?: {
    /** Request ID for tracking */
    requestId: string;
    
    /** User context */
    userId?: string;
    
    /** Request timestamp */
    timestamp: number;
  };
}

// ============================================================================
// UTILITY TYPES
// ============================================================================

/**
 * Type guard for schema objects
 */
export type SchemaTypeGuard = (obj: unknown) => obj is Schema;

/**
 * Type for schema filter functions
 */
export type SchemaFilter = (schema: Schema) => boolean;

/**
 * Type for schema sort functions
 */
export type SchemaSort = (a: Schema, b: Schema) => number;

/**
 * Redux action creator return types
 */
export type SchemaActionCreator<TPayload = unknown> = {
  type: string;
  payload: TPayload;
};

/**
 * Async thunk return types
 */
export type SchemaAsyncThunkReturn<TReturn = unknown> = Promise<TReturn>;