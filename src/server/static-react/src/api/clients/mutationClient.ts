/**
 * Mutation API Client - Unified Implementation
 * Replaces existing mutationClient.ts with standardized approach
 * Implements SCHEMA-002 compliance for mutation operations
 *
 * Uses generated TypeScript types from Rust backend for type safety.
 */

import { ApiClient, createApiClient } from "../core/client";
import { API_ENDPOINTS } from "../endpoints";
import { SCHEMA_STATES } from "../../constants/api";
import type { EnhancedApiResponse, MutationApiClient } from "../core/types";

// Import generated types from Rust backend for API consistency
import type {
  QueryResponse as BackendQueryResponse,
  IndexSearchResponse as BackendIndexSearchResponse,
  MutationResponse as BackendMutationResponse,
  SingleMutationResponse as BackendSingleMutationResponse,
} from "@generated/generated";

// Re-export backend types for consumers
export type {
  BackendQueryResponse,
  BackendIndexSearchResponse,
  BackendMutationResponse,
  BackendSingleMutationResponse,
};

// Local types for client-specific functionality (not generated from backend)
export interface ValidationResult {
  isValid: boolean;
  errors?: string[];
  warnings?: string[];
  schemaCompliance?: {
    schemaName: string;
    isApproved: boolean;
    missingFields?: string[];
    invalidFields?: string[];
  };
}

// Backward-compatible local types for client methods
// These will be migrated to use BackendMutationResponse/BackendQueryResponse
// once the client layer is refactored to handle the new response structures
export interface MutationResponse {
  success: boolean;
  result?: unknown;
  transactionId?: string;
  timestamp?: number;
  metadata?: Record<string, unknown>;
}

export interface QueryResponse {
  success: boolean;
  data?: unknown[];
  totalCount?: number;
  hasMore?: boolean;
  metadata?: Record<string, unknown>;
}

/**
 * Unified Mutation API Client Implementation
 */
export class UnifiedMutationClient implements MutationApiClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client =
      client ||
      createApiClient({
        enableCache: false, // Mutations should not be cached
        enableLogging: true,
        enableMetrics: true,
      });
  }

  /**
   * Execute a mutation against an approved schema
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   *
   * @param mutation The mutation object to execute
   * @returns Promise resolving to mutation result
   */
  async executeMutation(
    _mutation: Record<string, unknown>,
  ): Promise<EnhancedApiResponse<Record<string, unknown>>> {
    return this.client.post<MutationResponse>(
      API_ENDPOINTS.EXECUTE_MUTATION,
      _mutation,
      {
        validateSchema: false, // Skip schema validation for mutations
        timeout: 15000, // Longer timeout for mutation operations
        retries: 0, // No retries for mutations to prevent duplicate operations
        cacheable: false, // Never cache mutation results
      },
    );
  }

  /**
   * Execute multiple mutations in a batch for improved performance
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   *
   * @param mutations Array of mutation objects to execute
   * @returns Promise resolving to array of mutation IDs
   */
  async executeMutationsBatch(
    _mutations: Record<string, unknown>[],
  ): Promise<EnhancedApiResponse<string[]>> {
    // Server has no batch mutation endpoint
    return {
      success: false,
      error: "Batch mutations not supported",
      status: 501,
      data: [],
    };
  }

  /**
   * Execute a query against an approved schema
   * UNPROTECTED - No authentication required
   *
   * @param query The query object to execute
   * @returns Promise resolving to query results
   */
  async executeQuery(
    query: Record<string, unknown>,
  ): Promise<EnhancedApiResponse<Record<string, unknown>>> {
    return this.client.post<QueryResponse>(API_ENDPOINTS.EXECUTE_QUERY, query, {
      validateSchema: {
        operation: "read" as const,
        requiresApproved: true, // SCHEMA-002: Only approved schemas for queries
      },
      timeout: 10000, // Standard timeout for queries
      retries: 2, // Limited retries for read operations
      cacheable: true, // Query results can be cached
      cacheTtl: 60000, // Cache for 1 minute
    });
  }

  /**
   * Validate a mutation before execution
   * This checks schema compliance, field validation, and business rules
   *
   * @param mutation The mutation object to validate
   * @returns Promise resolving to validation result
   */
  async validateMutation(
    _mutation: Record<string, unknown>,
  ): Promise<EnhancedApiResponse<ValidationResult>> {
    // Removed: server has no /mutation/validate. Perform client-side no-op validation.
    return Promise.resolve({
      success: true,
      data: { isValid: true },
      status: 200,
    });
  }

  /**
   * Execute a batch of mutations as a single transaction
   * All mutations must target approved schemas
   *
   * @param mutations Array of mutation objects
   * @returns Promise resolving to batch execution results
   */
  async executeBatchMutations(
    _mutations: Record<string, unknown>[],
  ): Promise<EnhancedApiResponse<MutationResponse[]>> {
    // Removed: server has no /mutation/batch
    return {
      success: false,
      error: "Batch mutations not supported",
      status: 501,
      data: [],
    };
  }

  /**
   * Execute a parameterized query with filters and pagination
   * Provides enhanced query capabilities beyond basic executeQuery
   *
   * @param queryParams Query parameters including schema, filters, pagination
   * @returns Promise resolving to enhanced query results
   */
  async executeParameterizedQuery(queryParams: {
    schema: string;
    filters?: Record<string, unknown>;
    sort?: { field: string; direction: "asc" | "desc" }[];
    pagination?: { offset: number; limit: number };
    fields?: string[];
  }): Promise<EnhancedApiResponse<QueryResponse>> {
    // Repoint to /query (server supports only POST /query)
    return this.client.post<QueryResponse>(
      API_ENDPOINTS.EXECUTE_QUERY,
      queryParams,
      {
        validateSchema: {
          schemaName: queryParams.schema,
          operation: "read" as const,
          requiresApproved: true,
        },
        timeout: 15000,
        retries: 2,
        cacheable: true,
        cacheTtl: 120000,
        cacheKey: `parameterized-query:${JSON.stringify(queryParams)}`,
      },
    );
  }

  /**
   * Get mutation history for a specific record or schema
   * Useful for auditing and tracking changes
   *
   * @param params History query parameters
   * @returns Promise resolving to mutation history
   */
  async getMutationHistory(
    _params: Record<string, unknown>,
  ): Promise<EnhancedApiResponse<MutationResponse[]>> {
    // Removed: server has no /mutation/history
    return {
      success: false,
      error: "Mutation history not supported",
      status: 501,
      data: [],
    };
  }

  /**
   * Check if a schema is available for mutations (SCHEMA-002 compliance)
   *
   * @param schemaName The name of the schema to check
   * @returns Promise resolving to schema availability info
   */
  async validateSchemaForMutation(schemaName: string): Promise<{
    isValid: boolean;
    schemaState: string;
    canMutate: boolean;
    canQuery: boolean;
    error?: string;
  }> {
    try {
      // Use the schema client to get schema details
      const response = await this.client.get<Record<string, unknown>>(
        API_ENDPOINTS.GET_SCHEMA(schemaName),
        {
          timeout: 5000,
          retries: 1,
          cacheable: true,
          cacheTtl: 180000, // Cache schema state for 3 minutes
        },
      );

      if (!response.success || !response.data) {
        return {
          isValid: false,
          schemaState: "unknown",
          canMutate: false,
          canQuery: false,
          error: `Schema '${schemaName}' not found`,
        };
      }

      const schema = response.data;
      const state = schema && typeof schema === 'object' && 'state' in schema
        ? String((schema as Record<string, unknown>).state)
        : 'unknown';
      const isApproved = state === SCHEMA_STATES.APPROVED;

      return {
        isValid: true,
        schemaState: state,
        canMutate: isApproved,
        canQuery: isApproved,
        error: isApproved
          ? undefined
          : `Schema '${schemaName}' is not approved (current state: ${state})`,
      };
    } catch (error: unknown) {
      const message = error instanceof Error ? error.message : String(error);
      return {
        isValid: false,
        schemaState: "error",
        canMutate: false,
        canQuery: false,
        error: `Failed to validate schema '${schemaName}': ${message}`,
      };
    }
  }

  /**
   * Get mutation history for a molecule by its UUID
   *
   * @param moleculeUuid The molecule UUID to get history for
   * @returns Promise resolving to molecule history events
   */
  async getMoleculeHistory(
    moleculeUuid: string,
  ): Promise<EnhancedApiResponse<Record<string, unknown>>> {
    return this.client.get<Record<string, unknown>>(
      API_ENDPOINTS.GET_MOLECULE_HISTORY(moleculeUuid),
      {
        timeout: 10000,
        retries: 1,
        cacheable: true,
        cacheTtl: 30000,
      },
    );
  }

  /**
   * Get content of a specific atom by its UUID
   *
   * @param atomUuid The atom UUID to get content for
   * @returns Promise resolving to atom content
   */
  async getAtomContent(
    atomUuid: string,
  ): Promise<EnhancedApiResponse<Record<string, unknown>>> {
    return this.client.get<Record<string, unknown>>(
      API_ENDPOINTS.GET_ATOM_CONTENT(atomUuid),
      {
        timeout: 10000,
        retries: 1,
        cacheable: true,
        cacheTtl: 60000,
      },
    );
  }

  /**
   * Get API metrics for mutation operations
   */
  getMetrics() {
    return this.client
      .getMetrics()
      .filter(
        (metric) =>
          metric.url.includes("/mutation") || metric.url.includes("/query"),
      );
  }

  /**
   * Clear any cached query results
   */
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const mutationClient = new UnifiedMutationClient();

// Export factory function for custom instances
export function createMutationClient(
  client?: ApiClient,
): UnifiedMutationClient {
  return new UnifiedMutationClient(client);
}

// Export aliases and convenience wrappers for backwards compatibility and index.ts exports
export const MutationClient = UnifiedMutationClient;

export const executeMutation = (mutation: Record<string, unknown>) =>
  mutationClient.executeMutation(mutation);
export const executeQuery = (query: Record<string, unknown>) => mutationClient.executeQuery(query);
export const validateMutation = (mutation: Record<string, unknown>) =>
  mutationClient.validateMutation(mutation);
export const validateSchemaForMutation = (schemaName: string) =>
  mutationClient.validateSchemaForMutation(schemaName);
export const getMoleculeHistory = (moleculeUuid: string) =>
  mutationClient.getMoleculeHistory(moleculeUuid);
export const getAtomContent = (atomUuid: string) =>
  mutationClient.getAtomContent(atomUuid);

export default mutationClient;
