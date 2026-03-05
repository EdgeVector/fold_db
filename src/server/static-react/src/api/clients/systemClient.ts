// System API Client — handles logs, database reset, and status

import { ApiClient, createApiClient } from "../core/client";
import { API_ENDPOINTS } from "../endpoints";
import {
  API_TIMEOUTS,
  API_RETRIES,
  API_CACHE_TTL,
  CACHE_KEYS,
  API_CONFIG,
} from "../../constants/api";
import type { EnhancedApiResponse } from "../core/types";

// System-specific response types
export interface LogEntry {
  timestamp: number;
  level: "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR";
  event_type: string;
  message: string;
  user_id?: string;
  metadata?: Record<string, string>;
}

export interface LogsResponse {
  logs: LogEntry[];
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

export interface AutoIdentityResponse {
  user_id: string;
  user_hash: string;
  public_key: string;
}

export interface SystemStatusResponse {
  status: string;
  uptime: number;
  version?: string;
  // Schema service URL configured on the backend (undefined = local/embedded)
  schema_service_url?: string;
}

export interface NodeKeyResponse {
  success: boolean;
  private_key?: string;
  public_key?: string;
  message: string;
}

export interface DatabaseConfigDto {
  type: "local" | "dynamodb" | "s3";
  path?: string;
  table_name?: string;
  region?: string;
  user_id?: string;
  bucket?: string;
  prefix?: string;
  local_path?: string;
}

export interface DatabaseConfigRequest {
  database: DatabaseConfigDto;
}

export interface DatabaseConfigResponse {
  success: boolean;
  message: string;
  requires_restart: boolean;
}

export interface SetupStorageLocal {
  type: "local";
  path: string;
}

export interface SetupStorageExemem {
  type: "exemem";
  api_url: string;
  api_key: string;
}

export interface SetupRequest {
  storage?: SetupStorageLocal | SetupStorageExemem;
  schema_service_url?: string;
}

export interface SetupResponse {
  success: boolean;
  message: string;
}

export interface DatabaseStatusResponse {
  initialized: boolean;
  has_saved_config: boolean;
}

// Unified System API Client Implementation
export class UnifiedSystemClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client =
      client ||
      createApiClient({
        enableCache: false, // System operations should be fresh
        enableLogging: true,
        enableMetrics: true,
      });
  }

  // Get system logs (no auth required)
  async getLogs(since?: number): Promise<EnhancedApiResponse<LogsResponse>> {
    const url = since
      ? `${API_ENDPOINTS.LIST_LOGS}?since=${since}`
      : API_ENDPOINTS.LIST_LOGS;

    return this.client.get<LogsResponse>(url, {
      requiresAuth: false, // Logs are public for monitoring
      timeout: API_TIMEOUTS.STANDARD,
      retries: API_RETRIES.STANDARD,
      cacheable: false, // Always get fresh logs
    });
  }

  // Reset the database (destructive, requires auth)
  async resetDatabase(
    confirm: boolean = false,
  ): Promise<EnhancedApiResponse<ResetDatabaseResponse>> {
    if (!confirm) {
      throw new Error("Database reset requires explicit confirmation");
    }

    const request: ResetDatabaseRequest = { confirm };

    return this.client.post<ResetDatabaseResponse>(
      API_ENDPOINTS.RESET_DATABASE,
      request,
      {
        timeout: API_TIMEOUTS.DESTRUCTIVE_OPERATIONS, // Longer timeout for database operations
        retries: API_RETRIES.NONE, // No retries for destructive operations
        cacheable: false, // Never cache destructive operations
      },
    );
  }

  // Get system status and health information (no auth required)
  async getSystemStatus(): Promise<EnhancedApiResponse<SystemStatusResponse>> {
    return this.client.get<SystemStatusResponse>(
      API_ENDPOINTS.GET_SYSTEM_STATUS,
      {
        requiresAuth: false, // Status is public for monitoring
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.CRITICAL, // Multiple retries for critical system data
        cacheable: true,
        cacheTtl: API_CACHE_TTL.SYSTEM_STATUS, // Cache for 30 seconds
        cacheKey: CACHE_KEYS.SYSTEM_STATUS,
      },
    );
  }

  // Get the auto-identity (default user) for local dev (no auth required)
  async getAutoIdentity(): Promise<EnhancedApiResponse<AutoIdentityResponse>> {
    return this.client.get<AutoIdentityResponse>(API_ENDPOINTS.AUTO_IDENTITY, {
      requiresAuth: false,
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: false,
    });
  }

  // Get the node's private key (no auth required)
  async getNodePrivateKey(): Promise<EnhancedApiResponse<NodeKeyResponse>> {
    return this.client.get<NodeKeyResponse>(
      API_ENDPOINTS.GET_NODE_PRIVATE_KEY,
      {
        requiresAuth: false, // No authentication required for UI access
        timeout: API_TIMEOUTS.STANDARD,
        retries: API_RETRIES.STANDARD,
        cacheable: false, // Never cache private keys
      },
    );
  }

  // Get the node's public key (public, can be shared)
  async getNodePublicKey(): Promise<EnhancedApiResponse<NodeKeyResponse>> {
    return this.client.get<NodeKeyResponse>(API_ENDPOINTS.GET_NODE_PUBLIC_KEY, {
      requiresAuth: false, // Public key is safe to share
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: true,
      cacheTtl: API_CACHE_TTL.SYSTEM_STATUS, // Cache for 30 seconds
      cacheKey: CACHE_KEYS.SYSTEM_PUBLIC_KEY,
    });
  }

  // Create EventSource for log streaming (real-time log updates)
  createLogStream(
    onMessage: (message: string) => void,
    onError?: (error: Event) => void,
  ): EventSource {
    // Build URL manually using same logic as ApiClient.buildUrl()
    const endpoint = API_ENDPOINTS.STREAM_LOGS;
    const streamUrl = endpoint.startsWith("http")
      ? endpoint
      : `${API_CONFIG.BASE_URL}${endpoint.startsWith("/") ? "" : "/"}${endpoint}`;

    const eventSource = new EventSource(streamUrl);

    eventSource.onmessage = (event) => {
      onMessage(event.data);
    };

    if (onError) {
      eventSource.onerror = onError;
    }

    return eventSource;
  }

  // Validate reset database request (client-side validation helper)
  validateResetRequest(request: ResetDatabaseRequest): {
    isValid: boolean;
    errors: string[];
  } {
    const errors: string[] = [];

    if (typeof request !== "object" || request === null) {
      errors.push("Request must be an object");
      return { isValid: false, errors };
    }

    if (typeof request.confirm !== "boolean") {
      errors.push("Confirm must be a boolean value");
    } else if (!request.confirm) {
      errors.push("Confirm must be true to proceed with database reset");
    }

    return {
      isValid: errors.length === 0,
      errors,
    };
  }

  // Get API metrics for system operations
  getMetrics() {
    return this.client
      .getMetrics()
      .filter(
        (metric) =>
          metric.url.includes("/system") || metric.url.includes("/logs"),
      );
  }

  // Get database configuration (no auth required)
  async getDatabaseConfig(): Promise<EnhancedApiResponse<DatabaseConfigDto>> {
    return this.client.get<DatabaseConfigDto>(
      API_ENDPOINTS.GET_DATABASE_CONFIG,
      {
        requiresAuth: false,
        timeout: API_TIMEOUTS.STANDARD,
        retries: API_RETRIES.STANDARD,
        cacheable: true,
        cacheTtl: API_CACHE_TTL.SYSTEM_STATUS,
        cacheKey: "database_config",
      },
    );
  }

  // Update database configuration (no auth required)
  async updateDatabaseConfig(
    config: DatabaseConfigDto,
  ): Promise<EnhancedApiResponse<DatabaseConfigResponse>> {
    const request: DatabaseConfigRequest = { database: config };

    return this.client.post<DatabaseConfigResponse>(
      API_ENDPOINTS.UPDATE_DATABASE_CONFIG,
      request,
      {
        timeout: API_TIMEOUTS.STANDARD,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  // Apply setup configuration (storage and/or schema service URL)
  async applySetup(
    setup: SetupRequest,
  ): Promise<EnhancedApiResponse<SetupResponse>> {
    return this.client.post<SetupResponse>(API_ENDPOINTS.APPLY_SETUP, setup, {
      timeout: API_TIMEOUTS.STANDARD,
      retries: API_RETRIES.NONE,
      cacheable: false,
    });
  }

  // Migrate local database to the cloud (destructive)
  async migrateToCloud(
    apiUrl: string,
    apiKey: string,
  ): Promise<EnhancedApiResponse<any>> {
    return this.client.post<any>(
      API_ENDPOINTS.MIGRATE_TO_CLOUD,
      { api_url: apiUrl, api_key: apiKey },
      {
        timeout: API_TIMEOUTS.BATCH,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  // Get database initialization status (no auth required)
  async getDatabaseStatus(): Promise<EnhancedApiResponse<DatabaseStatusResponse>> {
    return this.client.get<DatabaseStatusResponse>(API_ENDPOINTS.GET_DATABASE_STATUS, {
      requiresAuth: false,
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: false,
    });
  }

  // Clear system-related cache
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
export const getAutoIdentity = systemClient.getAutoIdentity.bind(systemClient);
export const getSystemStatus = systemClient.getSystemStatus.bind(systemClient);
export const getNodePrivateKey =
  systemClient.getNodePrivateKey.bind(systemClient);
export const getNodePublicKey =
  systemClient.getNodePublicKey.bind(systemClient);
export const getDatabaseConfig =
  systemClient.getDatabaseConfig.bind(systemClient);
export const updateDatabaseConfig =
  systemClient.updateDatabaseConfig.bind(systemClient);
export const applySetup = systemClient.applySetup.bind(systemClient);
export const migrateToCloud = systemClient.migrateToCloud.bind(systemClient);
export const getDatabaseStatus = systemClient.getDatabaseStatus.bind(systemClient);
export const createLogStream = systemClient.createLogStream.bind(systemClient);
export const validateResetRequest =
  systemClient.validateResetRequest.bind(systemClient);

export default systemClient;
