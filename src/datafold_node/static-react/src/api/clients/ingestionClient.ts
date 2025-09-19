/**
 * Ingestion API Client - Unified Implementation
 * Handles AI-powered data ingestion, schema generation, and AI provider configuration
 * Part of API-STD-1 standardization initiative
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES } from '../../constants/api';
import type { EnhancedApiResponse } from '../core/types';

// Ingestion-specific response types
export interface IngestionStatus {
  enabled: boolean;
  configured: boolean;
  provider: 'OpenRouter' | 'Ollama';
  model: string;
  auto_execute_mutations: boolean;
  default_trust_distance: number;
}

export interface OpenRouterConfig {
  api_key: string;
  model: string;
  base_url: string;
}

export interface OllamaConfig {
  model: string;
  base_url: string;
}

export interface IngestionConfig {
  provider: 'OpenRouter' | 'Ollama';
  openrouter: OpenRouterConfig;
  ollama: OllamaConfig;
}

export interface ValidationRequest {
  [key: string]: unknown; // JSON data to validate - safer than any
}

export interface ValidationResponse {
  valid: boolean;
  error?: string;
  message?: string;
  suggestions?: string[];
  schema_inferred?: string;
}

export interface ProcessIngestionRequest {
  data: Record<string, unknown>;
  auto_execute: boolean;
  trust_distance: number;
  pub_key: string;
}

export interface ProcessIngestionResponse {
  success: boolean;
  error?: string;
  schema_created?: string;
  records_processed?: number;
  mutations_executed?: number;
  ai_analysis?: {
    schema_recommendations?: string[];
    data_quality_notes?: string[];
    execution_summary?: string;
  };
}

/**
 * Unified Ingestion API Client Implementation
 */
export class UnifiedIngestionClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client = client || createApiClient({
      enableCache: false, // Ingestion operations should not be cached
      enableLogging: true,
      enableMetrics: true
    });
  }

  /**
   * Get ingestion service status and configuration
   * UNPROTECTED - Status endpoint is public for health monitoring
   * 
   * @returns Promise resolving to ingestion service status
   */
  async getStatus(): Promise<EnhancedApiResponse<IngestionStatus>> {
    return this.client.get<IngestionStatus>(
      API_ENDPOINTS.INGESTION_STATUS,
      {
        requiresAuth: false, // Status endpoint is public
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.STANDARD,
        cacheable: false // Status should always be fresh
      }
    );
  }

  /**
   * Get ingestion configuration
   * UNPROTECTED - No authentication required
   * 
   * @returns Promise resolving to general ingestion configuration
   */
  async getConfig(): Promise<EnhancedApiResponse<IngestionConfig>> {
    return this.client.get<IngestionConfig>(
      API_ENDPOINTS.INGESTION_CONFIG,
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.STANDARD,
        cacheable: false // Config should not be cached for security
      }
    );
  }

  /**
   * Save AI provider configuration
   * UNPROTECTED - No authentication required
   * 
   * @param config The Ingestion configuration to save
   * @returns Promise resolving to save operation result
   */
  async saveConfig(config: IngestionConfig): Promise<EnhancedApiResponse<{ success: boolean; message: string }>> {
    return this.client.post<{ success: boolean; message: string }>(
      API_ENDPOINTS.INGESTION_CONFIG,
      config,
      {
        timeout: API_TIMEOUTS.CONFIG, // Longer timeout for config operations
        retries: API_RETRIES.LIMITED, // Limited retries for config changes
        cacheable: false // Never cache config operations
      }
    );
  }

  /**
   * Validate JSON data structure for ingestion
   * UNPROTECTED - Validation is a utility operation
   * 
   * @param data The JSON data to validate
   * @returns Promise resolving to validation result
   */
  async validateData(data: ValidationRequest): Promise<EnhancedApiResponse<ValidationResponse>> {
    return this.client.post<ValidationResponse>(
      API_ENDPOINTS.INGESTION_VALIDATE,
      data,
      {
        requiresAuth: false, // Validation is a utility operation
        timeout: API_TIMEOUTS.MUTATION, // Longer timeout for AI analysis
        retries: API_RETRIES.STANDARD,
        cacheable: false // Validation results should not be cached
      }
    );
  }

  /**
   * Process data ingestion with AI analysis
   * PROTECTED - Data processing requires authentication
   * 
   * @param data The data to process
   * @param options Processing options
   * @returns Promise resolving to processing result
   */
  async processIngestion(
    data: Record<string, unknown>,
    options: {
      autoExecute?: boolean;
      trustDistance?: number;
      pubKey?: string;
    } = {}
  ): Promise<EnhancedApiResponse<ProcessIngestionResponse>> {
    const request: ProcessIngestionRequest = {
      data,
      auto_execute: options.autoExecute ?? true,
      trust_distance: options.trustDistance ?? 0,
      pub_key: options.pubKey ?? 'default'
    };

    // Validate request before sending
    const validation = this.validateIngestionRequest(request);
    if (!validation.isValid) {
      throw new Error(`Invalid ingestion request: ${validation.errors.join(', ')}`);
    }

    return this.client.post<ProcessIngestionResponse>(
      API_ENDPOINTS.INGESTION_PROCESS,
      request,
      {
        timeout: API_TIMEOUTS.AI_PROCESSING, // Extended timeout for AI processing (60 seconds)
        retries: API_RETRIES.LIMITED, // Limited retries for processing operations
        cacheable: false // Processing results should not be cached
      }
    );
  }

  /**
   * Validate ingestion request before sending
   * Client-side validation helper
   * 
   * @param request The ingestion request to validate
   * @returns Validation result
   */
  validateIngestionRequest(request: ProcessIngestionRequest): {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  } {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Validate data
    if (!request.data || typeof request.data !== 'object') {
      errors.push('Data must be a valid object');
    } else if (Object.keys(request.data).length === 0) {
      errors.push('Data cannot be empty');
    }

    // Validate trust distance
    if (typeof request.trust_distance !== 'number' || request.trust_distance < 0) {
      errors.push('Trust distance must be a non-negative number');
    } else if (request.trust_distance > 10) {
      warnings.push('Trust distance is unusually high');
    }

    // Validate public key
    if (!request.pub_key || request.pub_key.trim().length === 0) {
      errors.push('Public key is required');
    }

    // Validate auto_execute flag
    if (typeof request.auto_execute !== 'boolean') {
      errors.push('Auto execute must be a boolean value');
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings
    };
  }

  /**
   * Create a properly structured ingestion request
   * Helper function for creating valid processing requests
   * 
   * @param data The data to process
   * @param options Processing configuration
   * @returns Ingestion request object
   */
  createIngestionRequest(
    data: Record<string, unknown>,
    options: {
      autoExecute?: boolean;
      trustDistance?: number;
      pubKey?: string;
    } = {}
  ): ProcessIngestionRequest {
    return {
      data: { ...data }, // Create a copy
      auto_execute: options.autoExecute ?? true,
      trust_distance: options.trustDistance ?? 0,
      pub_key: options.pubKey ?? 'default'
    };
  }

  /**
   * Get API metrics for ingestion operations
   */
  getMetrics() {
    return this.client.getMetrics().filter(metric => 
      metric.url.includes('/ingestion')
    );
  }

  /**
   * Clear ingestion-related cache (though ingestion operations should not be cached)
   */
  clearCache(): void {
    this.client.clearCache();
  }
}

// Create default instance
export const ingestionClient = new UnifiedIngestionClient();

// Export factory function for custom instances
export function createIngestionClient(client?: ApiClient): UnifiedIngestionClient {
  return new UnifiedIngestionClient(client);
}

// Named exports for backward compatibility and convenience
export const getStatus = ingestionClient.getStatus.bind(ingestionClient);
export const getConfig = ingestionClient.getConfig.bind(ingestionClient);
export const saveConfig = ingestionClient.saveConfig.bind(ingestionClient);
export const validateData = ingestionClient.validateData.bind(ingestionClient);
export const processIngestion = ingestionClient.processIngestion.bind(ingestionClient);

// Helper exports
export const validateIngestionRequest = ingestionClient.validateIngestionRequest.bind(ingestionClient);
export const createIngestionRequest = ingestionClient.createIngestionRequest.bind(ingestionClient);

// Type exports
export type {
  IngestionStatus,
  OpenRouterConfig,
  OllamaConfig,
  IngestionConfig,
  ValidationRequest,
  ValidationResponse,
  ProcessIngestionRequest,
  ProcessIngestionResponse
};

export default ingestionClient;
