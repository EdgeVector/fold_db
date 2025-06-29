/**
 * Mutation API Client - Unified Implementation
 * Replaces existing mutationClient.ts with standardized approach
 * Implements SCHEMA-002 compliance for mutation operations
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { SCHEMA_STATES } from '../../constants/api';
import type { EnhancedApiResponse, MutationApiClient } from '../core/types';
import type { SignedMessage } from '../../types/cryptography';

// Mutation-specific response types
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

/**
 * Unified Mutation API Client Implementation
 */
export class UnifiedMutationClient implements MutationApiClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: false, // Mutations should not be cached
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Execute a signed mutation against an approved schema
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   * 
   * @param signedMessage The signed mutation message containing payload and signature
   * @returns Promise resolving to mutation result
   */
  async executeMutation(signedMessage: SignedMessage): Promise<EnhancedApiResponse<MutationResponse>> {
    return this.client.post<MutationResponse>(
      API_ENDPOINTS.MUTATION,
      signedMessage,
      {
        requiresAuth: true,
        validateSchema: {
          operation: 'write' as const,
          requiresApproved: true // SCHEMA-002: Only approved schemas for mutations
        },
        timeout: 15000, // Longer timeout for mutation operations
        retries: 0, // No retries for mutations to prevent duplicate operations
        cacheable: false // Never cache mutation results
      }
    );
  }

  /**
   * Execute a signed query against an approved schema
   * PROTECTED - Requires authentication and SCHEMA-002 compliance
   * 
   * @param signedMessage The signed query message containing payload and signature
   * @returns Promise resolving to query results
   */
  async executeQuery(signedMessage: SignedMessage): Promise<EnhancedApiResponse<QueryResponse>> {
    return this.client.post<QueryResponse>(
      API_ENDPOINTS.QUERY,
      signedMessage,
      {
        requiresAuth: true,
        validateSchema: {
          operation: 'read' as const,
          requiresApproved: true // SCHEMA-002: Only approved schemas for queries
        },
        timeout: 10000, // Standard timeout for queries
        retries: 2, // Limited retries for read operations
        cacheable: true, // Query results can be cached
        cacheTtl: 60000 // Cache for 1 minute
      }
    );
  }

  /**
   * Validate a mutation before execution
   * This checks schema compliance, field validation, and business rules
   * 
   * @param mutation The mutation object to validate
   * @returns Promise resolving to validation result
   */
  async validateMutation(mutation: any): Promise<EnhancedApiResponse<ValidationResult>> {
    // Extract schema name from mutation if available
    const schemaName = mutation.schema || mutation.schemaName;
    
    return this.client.post<ValidationResult>(
      `${API_ENDPOINTS.MUTATION}/validate`,
      { mutation },
      {
        requiresAuth: true,
        validateSchema: schemaName ? {
          schemaName,
          operation: 'write' as const,
          requiresApproved: true
        } : true,
        timeout: 5000,
        retries: 1,
        cacheable: false
      }
    );
  }

  /**
   * Execute a batch of mutations as a single transaction
   * All mutations must target approved schemas
   * 
   * @param signedMessages Array of signed mutation messages
   * @returns Promise resolving to batch execution results
   */
  async executeBatchMutations(
    signedMessages: SignedMessage[]
  ): Promise<EnhancedApiResponse<MutationResponse[]>> {
    if (signedMessages.length === 0) {
      return {
        success: false,
        error: 'Empty batch: At least one mutation is required',
        status: 400,
        data: []
      };
    }

    return this.client.post<MutationResponse[]>(
      `${API_ENDPOINTS.MUTATION}/batch`,
      { mutations: signedMessages },
      {
        requiresAuth: true,
        validateSchema: {
          operation: 'write' as const,
          requiresApproved: true
        },
        timeout: 30000, // Extended timeout for batch operations
        retries: 0, // No retries for batch mutations
        cacheable: false
      }
    );
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
    filters?: Record<string, any>;
    sort?: { field: string; direction: 'asc' | 'desc' }[];
    pagination?: { offset: number; limit: number };
    fields?: string[];
  }): Promise<EnhancedApiResponse<QueryResponse>> {
    return this.client.post<QueryResponse>(
      `${API_ENDPOINTS.QUERY}/parameterized`,
      queryParams,
      {
        requiresAuth: true,
        validateSchema: {
          schemaName: queryParams.schema,
          operation: 'read' as const,
          requiresApproved: true
        },
        timeout: 15000,
        retries: 2,
        cacheable: true,
        cacheTtl: 120000, // Cache parameterized queries for 2 minutes
        cacheKey: `parameterized-query:${JSON.stringify(queryParams)}`
      }
    );
  }

  /**
   * Get mutation history for a specific record or schema
   * Useful for auditing and tracking changes
   * 
   * @param params History query parameters
   * @returns Promise resolving to mutation history
   */
  async getMutationHistory(params: {
    schema?: string;
    recordId?: string;
    limit?: number;
    offset?: number;
    startDate?: string;
    endDate?: string;
  }): Promise<EnhancedApiResponse<MutationResponse[]>> {
    const queryString = new URLSearchParams(
      Object.entries(params)
        .filter(([_, value]) => value !== undefined)
        .map(([key, value]) => [key, String(value)])
    ).toString();

    return this.client.get<MutationResponse[]>(
      `${API_ENDPOINTS.MUTATION}/history${queryString ? `?${queryString}` : ''}`,
      {
        requiresAuth: true,
        validateSchema: params.schema ? {
          schemaName: params.schema,
          operation: 'read' as const,
          requiresApproved: true
        } : true,
        timeout: 10000,
        retries: 2,
        cacheable: true,
        cacheTtl: 300000 // Cache history for 5 minutes
      }
    );
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
      const response = await this.client.get<any>(`/api/schemas/${schemaName}`, {
        requiresAuth: true,
        timeout: 5000,
        retries: 1,
        cacheable: true,
        cacheTtl: 180000 // Cache schema state for 3 minutes
      });

      if (!response.success || !response.data) {
        return {
          isValid: false,
          schemaState: 'unknown',
          canMutate: false,
          canQuery: false,
          error: `Schema '${schemaName}' not found`
        };
      }

      const schema = response.data;
      const isApproved = schema.state === SCHEMA_STATES.APPROVED;

      return {
        isValid: true,
        schemaState: schema.state,
        canMutate: isApproved,
        canQuery: isApproved,
        error: isApproved ? undefined : `Schema '${schemaName}' is not approved (current state: ${schema.state})`
      };
    } catch (error) {
      return {
        isValid: false,
        schemaState: 'error',
        canMutate: false,
        canQuery: false,
        error: `Failed to validate schema '${schemaName}': ${error.message}`
      };
    }
  }

  /**
   * Get API metrics for mutation operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(metric => 
      metric.url.includes('/mutation') || metric.url.includes('/query')
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
export function createMutationClient(client?: ApiClient): UnifiedMutationClient {
  return new UnifiedMutationClient(client);
}

// Backward compatibility exports - these will be deprecated
export const MutationClient = class {
  static async executeMutation(signedMessage: SignedMessage): Promise<EnhancedApiResponse<MutationResponse>> {
    return mutationClient.executeMutation(signedMessage);
  }

  static async executeQuery(signedMessage: SignedMessage): Promise<EnhancedApiResponse<QueryResponse>> {
    return mutationClient.executeQuery(signedMessage);
  }
};

// Export individual functions for backward compatibility
export const executeMutation = mutationClient.executeMutation.bind(mutationClient);
export const executeQuery = mutationClient.executeQuery.bind(mutationClient);
export const validateMutation = mutationClient.validateMutation.bind(mutationClient);
export const validateSchemaForMutation = mutationClient.validateSchemaForMutation.bind(mutationClient);

export default mutationClient;