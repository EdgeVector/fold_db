/**
 * Transform API Client - Unified Implementation
 * Handles transform operations and queue management
 * Part of API-STD-1 TASK-003 implementation
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import type { EnhancedApiResponse } from '../core/types';
// Import generated types from Rust - u64 fields are exported as numbers via #[ts(type = "number")]
import type { BackfillInfo, BackfillStatus } from '../../types/generated';

// Re-export for convenience
export type { BackfillInfo, BackfillStatus };

// Transform-specific response types
export interface Transform {
  id: string;
  schemaName: string;
  fieldName: string;
  logic: string;
  output: string;
  inputs?: string[];
  status?: 'pending' | 'processing' | 'completed' | 'failed';
  createdAt?: string;
  updatedAt?: string;
}

export interface TransformsResponse {
  data: Record<string, Transform> | Transform[];
  count?: number;
  timestamp?: number;
}

export interface QueueInfo {
  queue: string[];
  length: number;
  isEmpty: boolean;
  processing?: string[];
  completed?: string[];
  failed?: string[];
}

export interface AddToQueueRequest {
  transformId: string;
}

export interface AddToQueueResponse {
  success: boolean;
  message: string;
  transformId: string;
  queuePosition?: number;
  estimatedWaitTime?: number;
}

export interface TransformStatistics {
  field_value_sets: number;
  atom_creations: number;
  atom_updates: number;
  molecule_creations: number;
  molecule_updates: number;
  schema_loads: number;
  schema_changes: number;
  transform_triggers: number;
  transform_executions: number;
  transform_successes: number;
  transform_failures: number;
  transform_registrations: number;
  query_executions: number;
  mutation_executions: number;
  total_events: number;
  monitoring_start_time: number;
}

export interface BackfillStatistics {
  total_backfills: number;
  active_backfills: number;
  completed_backfills: number;
  failed_backfills: number;
  total_mutations_expected: number;
  total_mutations_completed: number;
  total_mutations_failed: number;
  total_records_produced: number;
}

/**
 * Unified Transform API Client Implementation
 */
export class UnifiedTransformClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: true, // Cache transform data for performance
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Get all available transforms
   * UNPROTECTED - No authentication required for reading transforms
   * Replaces TransformsTab fetch('/api/transforms')
   * 
   * @returns Promise resolving to transforms data
   */
  async getTransforms(): Promise<EnhancedApiResponse<TransformsResponse>> {
    return this.client.get<TransformsResponse>(API_ENDPOINTS.LIST_TRANSFORMS, {
      requiresAuth: false, // Transform reading is public
      timeout: 8000,
      retries: 2,
      cacheable: true,
      cacheTtl: 180000, // Cache for 3 minutes
      cacheKey: 'transforms:all'
    });
  }

  /**
   * Get current transform queue information
   * UNPROTECTED - No authentication required for queue monitoring
   * Replaces TransformsTab fetch('/api/transforms/queue')
   * 
   * @returns Promise resolving to queue status
   */
  async getQueue(): Promise<EnhancedApiResponse<QueueInfo>> {
    return this.client.get<QueueInfo>(API_ENDPOINTS.GET_TRANSFORM_QUEUE, {
      requiresAuth: false, // Queue monitoring is public
      timeout: 5000,
      retries: 3, // Multiple retries for critical queue data
      cacheable: false // Always get fresh queue data
    });
  }

  /**
   * Add a transform to the processing queue
   * UNPROTECTED - No authentication required for transform operations
   * Replaces TransformsTab fetch(`/api/transforms/queue/${transformId}`)
   * 
   * @param transformId - The ID of the transform to add to queue
   * @returns Promise resolving to queue addition result
   */
  async addToQueue(transformId: string): Promise<EnhancedApiResponse<AddToQueueResponse>> {
    if (!transformId || typeof transformId !== 'string') {
      throw new Error('Transform ID is required and must be a string');
    }

    return this.client.post<AddToQueueResponse>(
      API_ENDPOINTS.ADD_TO_TRANSFORM_QUEUE(transformId),
      undefined, // No body needed for this endpoint
      {
        timeout: 10000, // Longer timeout for queue operations
        retries: 1, // Limited retries for queue modifications
        cacheable: false // Never cache queue modification operations
      }
    );
  }

  /**
   * Refresh queue information (alias to getQueue for convenience)
   * This method provides semantic clarity for refresh operations
   * Used in TransformsTab for refreshing queue after adding transforms
   * 
   * @returns Promise resolving to current queue status
   */
  async refreshQueue(): Promise<EnhancedApiResponse<QueueInfo>> {
    return this.getQueue();
  }

  /**
   * Get all backfill information
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @returns Promise resolving to all backfill information
   */
  async getAllBackfills(): Promise<EnhancedApiResponse<BackfillInfo[]>> {
    return this.client.get<BackfillInfo[]>(API_ENDPOINTS.GET_ALL_BACKFILLS, {
      requiresAuth: false,
      timeout: 5000,
      retries: 2,
      cacheable: false
    });
  }

  /**
   * Get active (in-progress) backfills
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @returns Promise resolving to active backfill information
   */
  async getActiveBackfills(): Promise<EnhancedApiResponse<BackfillInfo[]>> {
    return this.client.get<BackfillInfo[]>(API_ENDPOINTS.GET_ACTIVE_BACKFILLS, {
      requiresAuth: false,
      timeout: 5000,
      retries: 2,
      cacheable: false
    });
  }

  /**
   * Get backfill information for a specific transform
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @param transformId - The ID of the transform
   * @returns Promise resolving to backfill information
   */
  async getBackfill(transformId: string): Promise<EnhancedApiResponse<BackfillInfo>> {
    if (!transformId || typeof transformId !== 'string') {
      throw new Error('Transform ID is required and must be a string');
    }

    return this.client.get<BackfillInfo>(API_ENDPOINTS.GET_BACKFILL(transformId), {
      requiresAuth: false,
      timeout: 5000,
      retries: 2,
      cacheable: false
    });
  }

  /**
   * Get transform execution statistics
   * UNPROTECTED - No authentication required for statistics monitoring
   * 
   * @returns Promise resolving to transform statistics
   */
  async getStatistics(): Promise<EnhancedApiResponse<TransformStatistics>> {
    return this.client.get<TransformStatistics>(API_ENDPOINTS.GET_TRANSFORM_STATISTICS, {
      requiresAuth: false,
      timeout: 5000,
      retries: 2,
      cacheable: false
    });
  }

  /**
   * Get backfill-specific statistics aggregated from all backfills
   * UNPROTECTED - No authentication required for backfill monitoring
   * 
   * @returns Promise resolving to backfill statistics
   */
  async getBackfillStatistics(): Promise<EnhancedApiResponse<BackfillStatistics>> {
    return this.client.get<BackfillStatistics>(API_ENDPOINTS.GET_BACKFILL_STATISTICS, {
      requiresAuth: false,
      timeout: 5000,
      retries: 2,
      cacheable: false
    });
  }

  /**
   * Get a specific transform by ID
   * UNPROTECTED - No authentication required for reading transform details
   * Future enhancement for detailed transform inspection
   * 
   * @param transformId - The ID of the transform to retrieve
   * @returns Promise resolving to transform details
   */
  async getTransform(transformId: string): Promise<EnhancedApiResponse<Transform>> {
    if (!transformId || typeof transformId !== 'string') {
      throw new Error('Transform ID is required and must be a string');
    }

    return this.client.get<Transform>(`${API_ENDPOINTS.LIST_TRANSFORMS}/${transformId}`, {
      requiresAuth: false, // Transform reading is public
      timeout: 5000,
      retries: 2,
      cacheable: true,
      cacheTtl: 300000, // Cache individual transforms for 5 minutes
      cacheKey: `transform:${transformId}`
    });
  }

  /**
   * Get API metrics for transform operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(metric => 
      metric.url.includes('/transforms') || metric.url.includes('/queue')
    );
  }

  /**
   * Clear transform-related cache
   */
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const transformClient = new UnifiedTransformClient();

// Export factory function for custom instances
export function createTransformClient(client?: ApiClient): UnifiedTransformClient {
  return new UnifiedTransformClient(client);
}

// Convenience exports for direct method access
export const getTransforms = transformClient.getTransforms.bind(transformClient);
export const getQueue = transformClient.getQueue.bind(transformClient);
export const addToQueue = transformClient.addToQueue.bind(transformClient);
export const refreshQueue = transformClient.refreshQueue.bind(transformClient);
export const getTransform = transformClient.getTransform.bind(transformClient);

export default transformClient;