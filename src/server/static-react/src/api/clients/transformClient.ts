/**
 * Transform API Client - Unified Implementation
 * Handles transform operations and queue management
 * Part of API-STD-1 TASK-003 implementation
 */

import { ApiClient, createApiClient } from "../core/client";
import { API_ENDPOINTS } from "../endpoints";
import type { EnhancedApiResponse } from "../core/types";
// Import generated types from Rust - u64 fields are exported as numbers via #[ts(type = "number")]
import type {
  BackfillInfo,
  BackfillStatus,
  Transform,
  BackfillStatistics,
} from "@generated/generated";

// Re-export for convenience
export type { BackfillInfo, BackfillStatus, Transform, BackfillStatistics };

// API response wrapper (the backend returns HashMap<String, Transform>)
export type TransformsResponse = Record<string, Transform>;

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

// Transform execution statistics (not yet derived from backend - TODO if needed)
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

/**
 * Unified Transform API Client Implementation
 */
export class UnifiedTransformClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client =
      client ||
      createApiClient({
        enableCache: true, // Cache transform data for performance
        enableLogging: true,
        enableMetrics: true,
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
      cacheKey: "transforms:all",
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
      cacheable: false, // Always get fresh queue data
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
  async addToQueue(
    transformId: string,
  ): Promise<EnhancedApiResponse<AddToQueueResponse>> {
    if (!transformId || typeof transformId !== "string") {
      throw new Error("Transform ID is required and must be a string");
    }

    return this.client.post<AddToQueueResponse>(
      API_ENDPOINTS.ADD_TO_TRANSFORM_QUEUE(transformId),
      undefined, // No body needed for this endpoint
      {
        timeout: 10000, // Longer timeout for queue operations
        retries: 1, // Limited retries for queue modifications
        cacheable: false, // Never cache queue modification operations
      },
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
      cacheable: false,
    });
  }

  /**
   * Get backfill information for a specific transform
   * UNPROTECTED - No authentication required for backfill monitoring
   *
   * @param transformId - The ID of the transform
   * @returns Promise resolving to backfill information
   */
  async getBackfill(
    transformId: string,
  ): Promise<EnhancedApiResponse<BackfillInfo>> {
    if (!transformId || typeof transformId !== "string") {
      throw new Error("Transform ID is required and must be a string");
    }

    return this.client.get<BackfillInfo>(
      API_ENDPOINTS.GET_BACKFILL(transformId),
      {
        requiresAuth: false,
        timeout: 5000,
        retries: 2,
        cacheable: false,
      },
    );
  }

  /**
   * Get a specific transform by ID from the transforms map
   * Note: The backend returns a map, so individual transform fetching
   * requires fetching all transforms and extracting the specific one
   *
   * @param transformId - The ID of the transform to retrieve
   * @returns Promise resolving to transform details
   */
  async getTransform(
    transformId: string,
  ): Promise<EnhancedApiResponse<Transform | null>> {
    if (!transformId || typeof transformId !== "string") {
      throw new Error("Transform ID is required and must be a string");
    }

    const result = await this.getTransforms();
    if (result.success && result.data) {
      const transform = result.data[transformId] || null;
      return {
        ...result,
        data: transform,
      };
    }
    return result as EnhancedApiResponse<null>;
  }

  /**
   * Get API metrics for transform operations
   */
  getMetrics() {
    return this.client
      .getMetrics()
      .filter(
        (metric) =>
          metric.url.includes("/transforms") || metric.url.includes("/queue"),
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
export function createTransformClient(
  client?: ApiClient,
): UnifiedTransformClient {
  return new UnifiedTransformClient(client);
}

// Convenience exports for direct method access
export const getTransforms =
  transformClient.getTransforms.bind(transformClient);
export const getQueue = transformClient.getQueue.bind(transformClient);
export const addToQueue = transformClient.addToQueue.bind(transformClient);
export const refreshQueue = transformClient.refreshQueue.bind(transformClient);
export const getTransform = transformClient.getTransform.bind(transformClient);

export default transformClient;
