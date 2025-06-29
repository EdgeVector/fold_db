/**
 * Ingestion API Client - Unified Implementation
 * Handles AI-powered data ingestion, schema generation, and OpenRouter configuration
 * Part of API-STD-1 standardization initiative
 */

import { ApiClient, createApiClient } from '../core/client';
import { API_ENDPOINTS } from '../endpoints';
import { API_TIMEOUTS, API_RETRIES, API_CACHE_TTL } from '../../constants/api';
import type { EnhancedApiResponse } from '../core/types';

// Ingestion-specific response types
export interface IngestionStatus {
  enabled: boolean;
  configured: boolean;
  model: string;
  auto_execute_mutations: boolean;
  default_trust_distance: number;
  last_activity?: string;
  api_key_set?: boolean;
}

export interface OpenRouterConfig {
  api_key: string;
  model: string;
  max_tokens?: number;
  temperature?: number;
}

export interface OpenRouterConfigResponse {
  api_key: string;
  model: string;
  max_tokens?: number;
  temperature?: number;
  last_updated?: string;
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
   * Get OpenRouter AI configuration
   * PROTECTED - Configuration access requires authentication
   * 
   * @returns Promise resolving to OpenRouter configuration
   */
  async getConfig(): Promise<EnhancedApiResponse<OpenRouterConfigResponse>> {
    return this.client.get<OpenRouterConfigResponse>(
      API_ENDPOINTS.INGESTION_CONFIG,
      {
        requiresAuth: true, // Configuration access requires auth
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.STANDARD,
        cacheable: false // Config should not be cached for security
      }
    );
  }

  /**
   * Save OpenRouter AI configuration
   * PROTECTED - Configuration changes require authentication
   * 
   * @param config The OpenRouter configuration to save
   * @returns Promise resolving to save operation result
   */
  async saveConfig(config: OpenRouterConfig): Promise<EnhancedApiResponse<{ success: boolean; message: string }>> {
    // Validate config before sending
    const validation = this.validateOpenRouterConfig(config);
    if (!validation.isValid) {
      throw new Error(`Invalid OpenRouter configuration: ${validation.errors.join(', ')}`);
    }

    return this.client.post<{ success: boolean; message: string }>(
      API_ENDPOINTS.INGESTION_CONFIG,
      config,
      {
        requiresAuth: true, // Config changes require auth
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
        requiresAuth: true, // Data processing requires auth
        timeout: API_TIMEOUTS.AI_PROCESSING, // Extended timeout for AI processing (60 seconds)
        retries: API_RETRIES.LIMITED, // Limited retries for processing operations
        cacheable: false // Processing results should not be cached
      }
    );
  }

  /**
   * Validate OpenRouter configuration before sending
   * Client-side validation helper
   * 
   * @param config The configuration to validate
   * @returns Validation result
   */
  validateOpenRouterConfig(config: OpenRouterConfig): {
    isValid: boolean;
    errors: string[];
    warnings: string[];
  } {
    const errors: string[] = [];
    const warnings: string[] = [];

    // Validate API key
    if (!config.api_key || config.api_key.trim().length === 0) {
      errors.push('API key is required');
    } else if (config.api_key.length < 10) {
      warnings.push('API key seems unusually short');
    }

    // Validate model
    if (!config.model || config.model.trim().length === 0) {
      errors.push('Model selection is required');
    }

    // Validate optional parameters
    if (config.max_tokens !== undefined) {
      if (typeof config.max_tokens !== 'number' || config.max_tokens <= 0) {
        errors.push('Max tokens must be a positive number');
      } else if (config.max_tokens > 32000) {
        warnings.push('Max tokens value is very high and may be expensive');
      }
    }

    if (config.temperature !== undefined) {
      if (typeof config.temperature !== 'number' || config.temperature < 0 || config.temperature > 2) {
        errors.push('Temperature must be a number between 0 and 2');
      }
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings
    };
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
   * Create a properly structured OpenRouter configuration
   * Helper function for creating valid configuration objects
   * 
   * @param apiKey The OpenRouter API key
   * @param model The AI model to use
   * @param options Additional configuration options
   * @returns OpenRouter configuration object
   */
  createOpenRouterConfig(
    apiKey: string,
    model: string = 'anthropic/claude-3.5-sonnet',
    options: {
      maxTokens?: number;
      temperature?: number;
    } = {}
  ): OpenRouterConfig {
    return {
      api_key: apiKey.trim(),
      model: model.trim(),
      ...(options.maxTokens && { max_tokens: options.maxTokens }),
      ...(options.temperature !== undefined && { temperature: options.temperature })
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
export const validateOpenRouterConfig = ingestionClient.validateOpenRouterConfig.bind(ingestionClient);
export const validateIngestionRequest = ingestionClient.validateIngestionRequest.bind(ingestionClient);
export const createOpenRouterConfig = ingestionClient.createOpenRouterConfig.bind(ingestionClient);
export const createIngestionRequest = ingestionClient.createIngestionRequest.bind(ingestionClient);

export default ingestionClient;