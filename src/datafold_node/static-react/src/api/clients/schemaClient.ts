/**
 * Schema API Client - Unified Implementation
 * Replaces existing schemaClient.ts with standardized approach
 * Implements SCHEMA-002 compliance at the API layer
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { SCHEMA_STATES, SCHEMA_OPERATIONS, API_TIMEOUTS, API_RETRIES, API_CACHE_TTL, CACHE_KEYS } from '../../constants/api';
import type { EnhancedApiResponse, SchemaApiClient } from '../core/types';
import type { Schema } from '../../types/schema';

// Schema-specific response types
export interface SchemasByStateResponse {
  data: string[];
  state: string;
}

export interface SchemasWithStateResponse {
  data: Record<string, string>;
}

export interface SchemaStatusResponse {
  available: number;
  approved: number;
  blocked: number;
  total: number;
}

/**
 * Unified Schema API Client Implementation
 */
export class UnifiedSchemaClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: true,
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Get all schemas with their current states
   * UNPROTECTED - No authentication required
   */
  async getSchemas(): Promise<EnhancedApiResponse<Schema[]>> {
    return this.client.get<Schema[]>(API_ENDPOINTS.SCHEMAS_BASE, {
      cacheable: true,
      cacheKey: 'schemas:all',
      cacheTtl: 300000 // 5 minutes
    });
  }

  /**
   * Get a specific schema by name
   * UNPROTECTED - No authentication required
   */
  async getSchema(name: string): Promise<EnhancedApiResponse<Schema>> {
    return this.client.get<Schema>(API_ENDPOINTS.SCHEMA_BY_NAME(name), {
      validateSchema: {
        schemaName: name,
        operation: 'read' as const,
        requiresApproved: false // Allow reading any schema for inspection
      },
      cacheable: true,
      cacheKey: `schema:${name}`,
      cacheTtl: 300000 // 5 minutes
    });
  }

  /**
   * Get schemas filtered by state (available, approved, blocked)
   * UNPROTECTED - No authentication required
   */
  async getSchemasByState(state: string): Promise<EnhancedApiResponse<SchemasByStateResponse>> {
    // Validate state parameter
    if (!Object.values(SCHEMA_STATES).includes(state as any)) {
      throw new Error(`Invalid schema state: ${state}. Must be one of: ${Object.values(SCHEMA_STATES).join(', ')}`);
    }

    return this.client.get<SchemasByStateResponse>(API_ENDPOINTS.SCHEMAS_BY_STATE(state), {
      cacheable: true,
      cacheKey: `schemas:state:${state}`,
      cacheTtl: 180000 // 3 minutes
    });
  }

  /**
   * Get all schemas with their state mappings
   * UNPROTECTED - No authentication required
   */
  async getAllSchemasWithState(): Promise<EnhancedApiResponse<SchemasWithStateResponse>> {
    return this.client.get<SchemasWithStateResponse>(API_ENDPOINTS.SCHEMAS_BASE, {
      cacheable: true,
      cacheKey: 'schemas:with-state',
      cacheTtl: 180000 // 3 minutes
    });
  }

  /**
   * Get schema status summary (counts by state)
   * UNPROTECTED - No authentication required
   */
  async getSchemaStatus(): Promise<EnhancedApiResponse<SchemaStatusResponse>> {
    return this.client.get<SchemaStatusResponse>(API_ENDPOINTS.SCHEMA_STATUS, {
      cacheable: true,
      cacheKey: 'schemas:status',
      cacheTtl: 60000 // 1 minute
    });
  }

  /**
   * Approve a schema (transition to approved state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only available schemas can be approved
   */
  async approveSchema(name: string): Promise<EnhancedApiResponse<void>> {
    return this.client.post<void>(
      API_ENDPOINTS.SCHEMA_APPROVE(name),
      {}, // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: name,
          operation: 'approve' as const,
          requiresApproved: false // Can approve non-approved schemas
        },
        timeout: 10000, // Longer timeout for state changes
        retries: 1 // Limited retries for state-changing operations
      }
    );
  }

  /**
   * Block a schema (transition to blocked state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only approved schemas can be blocked
   */
  async blockSchema(name: string): Promise<EnhancedApiResponse<void>> {
    return this.client.post<void>(
      API_ENDPOINTS.SCHEMA_BLOCK(name),
      {}, // Empty body, schema name is in URL
      {
        validateSchema: {
          schemaName: name,
          operation: 'block' as const,
          requiresApproved: true // Only approved schemas can be blocked
        },
        timeout: 10000, // Longer timeout for state changes
        retries: 1 // Limited retries for state-changing operations
      }
    );
  }

  /**
   * Get approved schemas only (SCHEMA-002 compliant)
   * This is a convenience method for components that need only approved schemas
   */
  async getApprovedSchemas(): Promise<EnhancedApiResponse<Schema[]>> {
    try {
      const response = await this.getSchemasByState(SCHEMA_STATES.APPROVED);
      
      if (!response.success || !response.data) {
        return {
          success: false,
          error: 'Failed to fetch approved schemas',
          status: response.status,
          data: []
        };
      }

      // Get full schema details for approved schemas
      const approvedSchemaNames = response.data.data;
      const schemaPromises = approvedSchemaNames.map(name => 
        this.getSchema(name).catch(error => ({
          success: false,
          error: error.message,
          status: error.status || 500,
          data: null
        }))
      );

      const schemaResponses = await Promise.all(schemaPromises);
      const successfulSchemas = schemaResponses
        .filter(resp => resp.success && resp.data)
        .map(resp => resp.data!);

      return {
        success: true,
        data: successfulSchemas,
        status: 200,
        meta: {
          requestId: response.meta?.requestId,
          timestamp: Date.now(),
          cached: response.meta?.cached || false
        }
      };
    } catch (error) {
      return {
        success: false,
        error: error.message || 'Failed to fetch approved schemas',
        status: error.status || 500,
        data: []
      };
    }
  }

  /**
   * Load a schema into memory
   * UNPROTECTED - No authentication required
   */
  async loadSchema(name: string): Promise<EnhancedApiResponse<void>> {
    return this.client.post<void>(
      API_ENDPOINTS.SCHEMA_LOAD(name),
      {}, // Empty body, schema name is in URL
      {
        timeout: 10000, // Longer timeout for load operations
        retries: 1 // Limited retries for state-changing operations
      }
    );
  }

  /**
   * Unload a schema from memory (remove/delete)
   * UNPROTECTED - No authentication required
   */
  async unloadSchema(name: string): Promise<EnhancedApiResponse<void>> {
    return this.client.delete<void>(API_ENDPOINTS.SCHEMA_UNLOAD(name), {
      timeout: 10000, // Longer timeout for unload operations
      retries: 1 // Limited retries for state-changing operations
    });
  }

  /**
   * Validate if a schema can be used for mutations/queries (SCHEMA-002 compliance)
   */
  async validateSchemaForOperation(
    schemaName: string,
    operation: 'mutation' | 'query'
  ): Promise<{ isValid: boolean; error?: string; schema?: Schema }> {
    try {
      const response = await this.getSchema(schemaName);
      
      if (!response.success || !response.data) {
        return {
          isValid: false,
          error: `Schema '${schemaName}' not found`
        };
      }

      const schema = response.data;
      
      // SCHEMA-002: Only approved schemas can be used for mutations and queries
      if (schema.state !== SCHEMA_STATES.APPROVED) {
        return {
          isValid: false,
          error: `Schema '${schemaName}' is not approved. Current state: ${schema.state}. Only approved schemas can be used for ${operation}s.`,
          schema
        };
      }

      return {
        isValid: true,
        schema
      };
    } catch (error) {
      return {
        isValid: false,
        error: `Failed to validate schema '${schemaName}': ${error.message}`
      };
    }
  }

  /**
   * Clear schema cache
   */
  clearCache(): void {
    this.client.clearCache();
  }

  /**
   * Get cache statistics
   */
  getCacheStats(): { size: number; hitRate: number } {
    return this.client.getCacheStats();
  }

  /**
   * Get API metrics
   */
  getMetrics() {
    return this.client.getMetrics();
  }
}

// Create default instance
export const schemaClient = new UnifiedSchemaClient();

// Export factory function for custom instances
export function createSchemaClient(client?: ApiClient): UnifiedSchemaClient {
  return new UnifiedSchemaClient(client);
}

// Backward compatibility exports - these will be deprecated
export const getSchemasByState = schemaClient.getSchemasByState.bind(schemaClient);
export const getAllSchemasWithState = schemaClient.getAllSchemasWithState.bind(schemaClient);
export const getSchemaStatus = schemaClient.getSchemaStatus.bind(schemaClient);
export const getSchema = schemaClient.getSchema.bind(schemaClient);
export const approveSchema = schemaClient.approveSchema.bind(schemaClient);
export const blockSchema = schemaClient.blockSchema.bind(schemaClient);

// New exports
export const loadSchema = schemaClient.loadSchema.bind(schemaClient);
export const unloadSchema = schemaClient.unloadSchema.bind(schemaClient);
export const getApprovedSchemas = schemaClient.getApprovedSchemas.bind(schemaClient);
export const validateSchemaForOperation = schemaClient.validateSchemaForOperation.bind(schemaClient);

export default schemaClient;