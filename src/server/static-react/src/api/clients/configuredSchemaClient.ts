/**
 * Configured Schema Client
 *
 * This module provides a schema client that respects the schema service environment
 * configuration (local/dev/prod). It creates an ApiClient with the appropriate
 * baseUrl based on the selected environment.
 */

import { createApiClient } from "../core/client";
import { UnifiedSchemaClient } from "./schemaClient";
import { SCHEMA_SERVICE_ENVIRONMENTS } from "../../contexts/SchemaServiceConfigContext";

/**
 * Get the current schema service base URL from localStorage
 * This function is synchronous and reads from localStorage directly
 */
export function getSchemaServiceBaseUrl(): string {
  const STORAGE_KEY = "schemaServiceEnvironment";
  const stored = localStorage.getItem(STORAGE_KEY);

  // Build environments map from the single source of truth
  const environments: Record<string, string> = Object.fromEntries(
    Object.values(SCHEMA_SERVICE_ENVIRONMENTS).map((env) => [
      env.id,
      env.baseUrl,
    ])
  );

  return (
    environments[stored || "local"] || SCHEMA_SERVICE_ENVIRONMENTS.LOCAL.baseUrl
  );
}

/**
 * Create a configured schema client instance
 * This creates a new ApiClient with the appropriate baseUrl based on
 * the current schema service environment selection
 */
export function createConfiguredSchemaClient(): UnifiedSchemaClient {
  const baseUrl = getSchemaServiceBaseUrl();

  // Create ApiClient with configured baseUrl
  const apiClient = createApiClient({
    baseUrl: baseUrl || "/api", // Use '/api' as default for local
    enableCache: true,
    enableLogging: true,
    enableMetrics: true,
  });

  // Create and return schema client with configured API client
  return new UnifiedSchemaClient(apiClient);
}

// Create the default configured instance
// This will be recreated when environment changes
let configuredSchemaClientInstance: UnifiedSchemaClient | null = null;

/**
 * Get the configured schema client singleton
 * This returns a singleton instance that respects the current environment configuration
 */
export function getConfiguredSchemaClient(): UnifiedSchemaClient {
  // Always create a fresh instance to pick up configuration changes
  // The ApiClient has its own caching mechanism, so this is safe
  configuredSchemaClientInstance = createConfiguredSchemaClient();
  return configuredSchemaClientInstance;
}

/**
 * Force recreation of the schema client instance
 * Call this when the environment configuration changes
 */
export function resetSchemaClient(): void {
  configuredSchemaClientInstance = null;
}

/**
 * Check the status of a schema service endpoint
 * @param baseUrl - The base URL to check
 * @returns Promise with status information
 */
export async function checkSchemaServiceStatus(baseUrl: string): Promise<{
  success: boolean;
  status?: string;
  error?: string;
  responseTime?: number;
}> {
  const startTime = Date.now();

  // Local schema service uses /health endpoint (baseUrl already includes /api)
  // AWS services use /schema with POST {"action": "status"}
  const isLocal =
    baseUrl.includes("127.0.0.1") || baseUrl.includes("localhost");
  const url = isLocal ? `${baseUrl}/health` : `${baseUrl}/schema`;

  try {
    const options: RequestInit = {
      method: isLocal ? "GET" : "POST",
      headers: {
        "Content-Type": "application/json",
      },
      signal: AbortSignal.timeout(5000), // 5 second timeout
    };

    // Add body only for AWS endpoints
    if (!isLocal) {
      options.body = JSON.stringify({ action: "status" });
    }

    const response = await fetch(url, options);
    const responseTime = Date.now() - startTime;

    if (response.ok) {
      const data = await response.json();
      return {
        success: true,
        status: data.status || "online",
        responseTime,
      };
    } else {
      return {
        success: false,
        error: `HTTP ${response.status}: ${response.statusText}`,
        responseTime,
      };
    }
  } catch (error: any) {
    const responseTime = Date.now() - startTime;
    return {
      success: false,
      error:
        error.name === "TimeoutError" ? "Connection timeout" : error.message,
      responseTime,
    };
  }
}

// Export a default instance getter
export default getConfiguredSchemaClient;
