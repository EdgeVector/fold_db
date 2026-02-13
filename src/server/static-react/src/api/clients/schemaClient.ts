/**
 * Schema API Client - Unified Implementation
 * Replaces existing schemaClient.ts with standardized approach
 * Implements SCHEMA-002 compliance at the API layer
 */

import { ApiClient, getSharedClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { SCHEMA_STATES, SCHEMA_OPERATIONS, API_TIMEOUTS, API_RETRIES, API_CACHE_TTL, CACHE_KEYS } from '../../constants/api';
import type { EnhancedApiResponse, SchemaApiClient } from '../core/types';
import type { Schema, SchemaState } from '../../types/schema';
import { normalizeSchemaState } from '../../utils/rangeSchemaHelpers.js';

// Schema-specific response types
export interface SchemasByStateResponse {
  data: string[];
  state: string;
}

export type SchemasWithStateResponse = Record<string, SchemaState>;

const FALLBACK_SCHEMA_STATE = SCHEMA_STATES.AVAILABLE as SchemaState;
const RECOGNIZED_SCHEMA_STATES: ReadonlySet<SchemaState> = new Set<SchemaState>([
  SCHEMA_STATES.AVAILABLE as SchemaState,
  SCHEMA_STATES.APPROVED as SchemaState,
  SCHEMA_STATES.BLOCKED as SchemaState,
  'loading',
  'error'
]);

function extractSchemaName(schema: unknown): string | null {
  if (!schema || typeof schema !== 'object') {
    return null;
  }

  const candidate = (schema as { name?: unknown }).name;
  if (typeof candidate === 'string' && candidate.trim().length > 0) {
    return candidate;
  }

  const nested = (schema as { schema?: { name?: unknown } }).schema;
  if (nested && typeof nested === 'object') {
    const nestedName = (nested as { name?: unknown }).name;
    if (typeof nestedName === 'string' && nestedName.trim().length > 0) {
      return nestedName;
    }
  }

  return null;
}

function extractRawSchemaState(schema: unknown): unknown {
  if (!schema || typeof schema !== 'object') {
    return undefined;
  }

  const candidates: unknown[] = [
    (schema as { state?: unknown }).state,
    (schema as { schema_state?: unknown }).schema_state,
    (schema as { schemaState?: unknown }).schemaState,
    (schema as { status?: unknown }).status,
    (schema as { current_state?: unknown }).current_state,
    (schema as { schema?: { state?: unknown } }).schema?.state
  ];

  return candidates.find((candidate) => candidate !== undefined);
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
    this.client = client || getSharedClient();
  }

  /**
   * Get all schemas with their current states
   * UNPROTECTED - No authentication required
   */
  async getSchemas(): Promise<EnhancedApiResponse<Schema[]>> {
    const response = await this.client.get<unknown>(API_ENDPOINTS.LIST_SCHEMAS, {
      cacheable: true,
      cacheKey: 'schemas:all',
      cacheTtl: 300000 // 5 minutes
    });

    if (!response.success) {
      return { ...response, data: [] } as EnhancedApiResponse<Schema[]>;
    }

    // Normalize response into Schema[]
    // The backend returns { ok: true, schemas: [...], count: N, user_hash: '...' }
    const raw = (response as any).data;
    let list: Schema[] = [];

    if (raw && typeof raw === 'object' && 'schemas' in raw) {
      // Extract schemas from response envelope
      const schemas = raw.schemas;
      if (Array.isArray(schemas)) {
        list = schemas as Schema[];
      } else if (schemas && typeof schemas === 'object') {
        list = Object.values(schemas as Record<string, Schema>);
      }
    } else if (Array.isArray(raw)) {
      // Direct array response (legacy format)
      list = raw as Schema[];
    } else if (raw && typeof raw === 'object') {
      // Server may return an object map from name -> Schema
      // Convert to array of Schema objects
      list = Object.values(raw as Record<string, Schema>);
    } else {
      // Unexpected shape; log once and return empty list to keep UI stable
      if (typeof console !== 'undefined' && console.warn) {
        console.warn('[schemaClient.getSchemas] Unexpected response shape; normalizing to empty array', raw);
      }
      list = [];
    }

    return { ...response, data: list } as EnhancedApiResponse<Schema[]>;
  }

  /**
   * Get a specific schema by name
   * UNPROTECTED - No authentication required
   */
  async getSchema(name: string): Promise<EnhancedApiResponse<Schema>> {
    return this.client.get<Schema>(API_ENDPOINTS.GET_SCHEMA(name), {
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
   * Get schemas filtered by state (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemasByState(state: string): Promise<EnhancedApiResponse<SchemasByStateResponse>> {
    if (!Object.values(SCHEMA_STATES).includes(state as any)) {
      throw new Error(`Invalid schema state: ${state}. Must be one of: ${Object.values(SCHEMA_STATES).join(', ')}`);
    }
    const all = await this.getSchemas();
    if (!all.success || !all.data) {
      return { success: false, error: 'Failed to fetch schemas', status: all.status, data: { data: [], state } };
    }
    const names = (all.data as any[])
      .filter((s: any) => s.state === state)
      .map((s: any) => s.name);
    return {
      success: true,
      data: { data: names, state },
      status: 200,
      meta: { timestamp: Date.now(), cached: all.meta?.cached || false }
    };
  }

  /**
   * Get all schemas with their state mappings (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getAllSchemasWithState(): Promise<EnhancedApiResponse<SchemasWithStateResponse>> {
    const all = await this.getSchemas();
    if (!all.success || !all.data) {
      return {
        success: false,
        error: 'Failed to fetch schemas',
        status: all.status,
        data: {} as SchemasWithStateResponse
      };
    }

    const schemas = Array.isArray(all.data) ? all.data : [];
    const states: SchemasWithStateResponse = {};

    schemas.forEach((schemaEntry) => {
      const name = extractSchemaName(schemaEntry);
      if (!name) {
        if (typeof console !== 'undefined' && console.warn) {
          console.warn('[schemaClient.getAllSchemasWithState] Encountered schema entry without a name, skipping entry.');
        }
        return;
      }

      const rawState = extractRawSchemaState(schemaEntry);
      const normalized = normalizeSchemaState(rawState) as SchemaState;

      if (!rawState || normalized.length === 0) {
        if (typeof console !== 'undefined' && console.warn) {
          console.warn(
            `[schemaClient.getAllSchemasWithState] Missing schema state for '${name}', defaulting to '${FALLBACK_SCHEMA_STATE}'.`
          );
        }
        states[name] = FALLBACK_SCHEMA_STATE;
        return;
      }

      if (!RECOGNIZED_SCHEMA_STATES.has(normalized)) {
        if (typeof console !== 'undefined' && console.warn) {
          console.warn(
            `[schemaClient.getAllSchemasWithState] Unrecognized schema state '${String(rawState)}' for '${name}', defaulting to '${FALLBACK_SCHEMA_STATE}'.`
          );
        }
        states[name] = FALLBACK_SCHEMA_STATE;
        return;
      }

      states[name] = normalized;
    });

    return {
      success: true,
      data: states,
      status: all.status ?? 200,
      meta: {
        ...all.meta,
        timestamp: Date.now(),
        cached: all.meta?.cached ?? false
      }
    };
  }

  /**
   * Get schema status summary (computed client-side)
   * UNPROTECTED - No authentication required
   */
  async getSchemaStatus(): Promise<EnhancedApiResponse<SchemaStatusResponse>> {
    const all = await this.getSchemas();
    if (!all.success || !all.data) {
      return { success: false, error: 'Failed to fetch schemas', status: all.status, data: { available: 0, approved: 0, blocked: 0, total: 0 } };
    }
    const list = all.data as any[];
    const counts = {
      available: list.filter(s => s.state === SCHEMA_STATES.AVAILABLE).length,
      approved: list.filter(s => s.state === SCHEMA_STATES.APPROVED).length,
      blocked: list.filter(s => s.state === SCHEMA_STATES.BLOCKED).length,
      total: list.length
    };
    return { success: true, data: counts, status: 200, meta: { timestamp: Date.now(), cached: all.meta?.cached || false } };
  }

  /**
   * Approve a schema (transition to approved state)
   * UNPROTECTED - No authentication required
   * SCHEMA-002 Compliance: Only available schemas can be approved
   */
  async approveSchema(name: string): Promise<EnhancedApiResponse<{ backfill_hash?: string }>> {
    return this.client.post<{ backfill_hash?: string }>(
      API_ENDPOINTS.APPROVE_SCHEMA(name),
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
      API_ENDPOINTS.BLOCK_SCHEMA(name),
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
      const response = await this.getSchemas();
      if (!response.success || !response.data) {
        return { success: false, error: 'Failed to fetch schemas', status: response.status, data: [] };
      }
      const approved = response.data.filter((s: any) => s.state === SCHEMA_STATES.APPROVED);
      return { success: true, data: approved, status: 200, meta: { timestamp: Date.now(), cached: response.meta?.cached } };
    } catch (error) {
      return { success: false, error: error.message || 'Failed to fetch approved schemas', status: error.status || 500, data: [] };
    }
  }

  /**
   * Load a schema into memory (no-op client-side; server has no endpoint)
   */
  async loadSchema(_name: string): Promise<EnhancedApiResponse<void>> {
    return { success: true, status: 200 } as any;
  }

  /**
   * Unload a schema from memory (no-op client-side; server has no endpoint)
   */
  async unloadSchema(_name: string): Promise<EnhancedApiResponse<void>> {
    return { success: true, status: 200 } as any;
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

  /**
   * Get backfill status by hash
   * UNPROTECTED - No authentication required
   */
  async getBackfillStatus(backfillHash: string): Promise<EnhancedApiResponse<any>> {
    return this.client.get<any>(`/api/backfill/${backfillHash}`, {
      cacheable: false, // Don't cache backfill status as it changes frequently
      timeout: 5000
    });
  }
}

// Create default instance
export const schemaClient = new UnifiedSchemaClient();

// Export factory function for custom instances
export function createSchemaClient(client?: ApiClient): UnifiedSchemaClient {
  return new UnifiedSchemaClient(client);
}


// Exported standalone functions
export const getSchemasByState = schemaClient.getSchemasByState.bind(schemaClient);
export const getAllSchemasWithState = schemaClient.getAllSchemasWithState.bind(schemaClient);
export const getSchemaStatus = schemaClient.getSchemaStatus.bind(schemaClient);
export const getSchema = schemaClient.getSchema.bind(schemaClient);
export const approveSchema = schemaClient.approveSchema.bind(schemaClient);
export const blockSchema = schemaClient.blockSchema.bind(schemaClient);
export const loadSchema = schemaClient.loadSchema.bind(schemaClient);
export const unloadSchema = schemaClient.unloadSchema.bind(schemaClient);
export const getApprovedSchemas = schemaClient.getApprovedSchemas.bind(schemaClient);
export const getBackfillStatus = schemaClient.getBackfillStatus.bind(schemaClient);

export default schemaClient;
