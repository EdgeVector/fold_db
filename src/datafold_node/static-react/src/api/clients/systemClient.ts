/**
 * System API Client - Unified Implementation
 * Handles system operations like logs, database reset, and status
 * Part of API-STD-1 TASK-002 implementation
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import type { EnhancedApiResponse } from '../core/types';

// System-specific response types
export interface LogsResponse {
  logs: string[];
  count?: number;
  timestamp?: number;
}

export interface ResetDatabaseRequest {
  confirm: boolean;
}

export interface ResetDatabaseResponse {
  success: boolean;
  message: string;
  timestamp?: number;
  affected_rows?: number;
}

export interface SystemStatusResponse {
  status: 'healthy' | 'degraded' | 'unhealthy';
  uptime: number;
  version?: string;
  node_id?: string;
  last_activity?: number;
  database: {
    connected: boolean;
    schemas_count?: number;
    size?: number;
  };
  network: {
    peers_count?: number;
    status?: string;
  };
  memory: {
    used?: number;
    total?: number;
    percentage?: number;
  };
}

/**
 * Unified System API Client Implementation
 */
export class UnifiedSystemClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: false, // System operations should be fresh
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Get system logs
   * UNPROTECTED - No authentication required
   * Replaces LogSidebar direct fetch('/api/logs')
   * 
   * @returns Promise resolving to logs array
   */
  async getLogs(): Promise<EnhancedApiResponse<LogsResponse>> {
    return this.client.get<LogsResponse>(API_ENDPOINTS.SYSTEM_LOGS, {
      requiresAuth: false, // Logs are public for monitoring
      timeout: 8000,
      retries: 2,
      cacheable: false // Always get fresh logs
    });
  }

  /**
   * Reset the database (destructive operation)
   * PROTECTED - Requires authentication for security
   * Replaces StatusSection direct fetch('/api/system/reset-database')
   * 
   * @param confirm - Confirmation flag (must be true)
   * @returns Promise resolving to reset result
   */
  async resetDatabase(confirm: boolean = false): Promise<EnhancedApiResponse<ResetDatabaseResponse>> {
    if (!confirm) {
      throw new Error('Database reset requires explicit confirmation');
    }

    const request: ResetDatabaseRequest = { confirm };

    return this.client.post<ResetDatabaseResponse>(
      API_ENDPOINTS.SYSTEM_RESET_DATABASE,
      request,
      {
        requiresAuth: true, // Destructive operation requires auth
        timeout: 30000, // Longer timeout for database operations
        retries: 0, // No retries for destructive operations
        cacheable: false // Never cache destructive operations
      }
    );
  }

  /**
   * Get system status and health information
   * UNPROTECTED - No authentication required for status monitoring
   * Future endpoint for system monitoring
   * 
   * @returns Promise resolving to system status
   */
  async getSystemStatus(): Promise<EnhancedApiResponse<SystemStatusResponse>> {
    return this.client.get<SystemStatusResponse>(API_ENDPOINTS.SYSTEM_STATUS, {
      requiresAuth: false, // Status is public for monitoring
      timeout: 5000,
      retries: 3, // Multiple retries for critical system data
      cacheable: true,
      cacheTtl: 30000, // Cache for 30 seconds
      cacheKey: 'system-status'
    });
  }

  /**
   * Create EventSource for log streaming
   * Helper method for components that need real-time log updates
   * This doesn't use the unified client as EventSource has different semantics
   * 
   * @param onMessage - Callback for new log messages
   * @param onError - Callback for connection errors
   * @returns EventSource instance (caller must close it)
   */
  createLogStream(
    onMessage: (message: string) => void,
    onError?: (error: Event) => void
  ): EventSource {
    const eventSource = new EventSource(API_ENDPOINTS.SYSTEM_LOGS_STREAM);
    
    eventSource.onmessage = (event) => {
      onMessage(event.data);
    };

    if (onError) {
      eventSource.onerror = onError;
    }

    return eventSource;
  }

  /**
   * Validate reset database request
   * Client-side validation helper
   * 
   * @param request - Reset request to validate
   * @returns Validation result
   */
  validateResetRequest(request: ResetDatabaseRequest): {
    isValid: boolean;
    errors: string[];
  } {
    const errors: string[] = [];

    if (typeof request !== 'object' || request === null) {
      errors.push('Request must be an object');
      return { isValid: false, errors };
    }

    if (typeof request.confirm !== 'boolean') {
      errors.push('Confirm must be a boolean value');
    } else if (!request.confirm) {
      errors.push('Confirm must be true to proceed with database reset');
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  }

  /**
   * Get API metrics for system operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(metric => 
      metric.url.includes('/system') || metric.url.includes('/logs')
    );
  }

  /**
   * Clear system-related cache
   */
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const systemClient = new UnifiedSystemClient();

// Export factory function for custom instances
export function createSystemClient(client?: ApiClient): UnifiedSystemClient {
  return new UnifiedSystemClient(client);
}

// Convenience exports for direct method access
export const getLogs = systemClient.getLogs.bind(systemClient);
export const resetDatabase = systemClient.resetDatabase.bind(systemClient);
export const getSystemStatus = systemClient.getSystemStatus.bind(systemClient);
export const createLogStream = systemClient.createLogStream.bind(systemClient);
export const validateResetRequest = systemClient.validateResetRequest.bind(systemClient);

export default systemClient;