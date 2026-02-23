/**
 * Ingestion API Client - Unified Implementation
 * Handles AI-powered data ingestion, schema generation, and AI provider configuration
 * Part of API-STD-1 standardization initiative
 */

import { ApiClient, createApiClient } from "../core/client";
import { API_ENDPOINTS, API_BASE_URLS } from "../endpoints";
import { API_TIMEOUTS, API_RETRIES, CONTENT_TYPES } from "../../constants/api";
import type { EnhancedApiResponse } from "../core/types";

// Ingestion-specific response types
export interface IngestionStatus {
  enabled: boolean;
  configured: boolean;
  provider: "OpenRouter" | "Ollama";
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
  provider: "OpenRouter" | "Ollama";
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
  progress_id: string;
}
// ... (interface continues, but we are just replacing the request construction part mostly)

// ...

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
  progress_id?: string; // ID for tracking progress
}

// Smart Folder types
export interface FileRecommendation {
  path: string;
  should_ingest: boolean;
  category: string;
  reason: string;
  file_size_bytes: number;
  estimated_cost: number;
}

export interface SmartFolderScanResponse {
  success: boolean;
  total_files: number;
  recommended_files: FileRecommendation[];
  skipped_files: FileRecommendation[];
  summary: Record<string, number>;
  total_estimated_cost: number;
  scan_truncated: boolean;
  max_depth_used: number;
  max_files_used: number;
}

export interface SmartFolderIngestResponse {
  success: boolean;
  batch_id: string;
  files_found: number;
  file_progress_ids: { file_name: string; progress_id: string }[];
  message: string;
}

// Batch status types
export interface BatchStatusResponse {
  batch_id: string;
  status: "Running" | "Paused" | "Completed" | "Cancelled" | "Failed";
  spend_limit: number | null;
  accumulated_cost: number;
  files_total: number;
  files_completed: number;
  files_failed: number;
  files_remaining: number;
  estimated_remaining_cost: number;
  current_file_name: string | null;
  current_file_step: string | null;
  current_file_progress: number | null;
}

// Progress tracking types
export interface IngestionProgress {
  id: string;
  current_step: string;
  progress_percentage: number;
  status_message: string;
  is_complete: boolean;
  is_failed: boolean;
  error_message?: string;
  started_at: string;
  completed_at?: string;
  results?: IngestionResults;
}

export interface IngestionResults {
  schema_name: string;
  new_schema_created: boolean;
  mutations_generated: number;
  mutations_executed: number;
}

export interface FileUploadResponse {
  success: boolean;
  error?: string;
  schema_name?: string;
  schema_used?: string;
  new_schema_created?: boolean;
  mutations_generated?: number;
  mutations_executed?: number;
}

/**
 * Unified Ingestion API Client Implementation
 */
export class UnifiedIngestionClient {
  private readonly client: ApiClient;

  constructor(client?: ApiClient) {
    this.client =
      client ||
      createApiClient({
        baseUrl: API_BASE_URLS.ROOT,
        enableCache: false, // Ingestion operations should not be cached
        enableLogging: true,
        enableMetrics: true,
      });
  }

  /**
   * Get ingestion service status and configuration
   * UNPROTECTED - Status endpoint is public for health monitoring
   *
   * @returns Promise resolving to ingestion service status
   */
  async getStatus(): Promise<EnhancedApiResponse<IngestionStatus>> {
    return this.client.get<IngestionStatus>(API_ENDPOINTS.GET_STATUS, {
      requiresAuth: false, // Status endpoint is public
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: false, // Status should always be fresh
    });
  }

  /**
   * Get all active ingestion progress
   * UNPROTECTED - Progress status is public for monitoring
   *
   * @returns Promise resolving to array of ingestion progress items
   */
  async getAllProgress(): Promise<EnhancedApiResponse<IngestionProgress[]>> {
    return this.client.get<IngestionProgress[]>("/ingestion/progress", {
      requiresAuth: false,
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: false, // Progress should always be fresh
    });
  }

  /**
   * Get progress for a specific job by ID
   * UNPROTECTED - Progress status is public for monitoring
   *
   * @param jobId The job ID to get progress for
   * @returns Promise resolving to the job progress
   */
  async getJobProgress(jobId: string): Promise<
    EnhancedApiResponse<{
      id: string;
      job_type: string;
      current_step: string;
      progress_percentage: number;
      status_message: string;
      is_complete: boolean;
      is_failed: boolean;
      error_message?: string;
      results?: Record<string, unknown>;
      started_at: number;
      completed_at?: number;
    }>
  > {
    return this.client.get(`/ingestion/progress/${jobId}`, {
      requiresAuth: false,
      timeout: API_TIMEOUTS.QUICK,
      retries: API_RETRIES.STANDARD,
      cacheable: false, // Progress should always be fresh
    });
  }

  /**
   * Get ingestion configuration
   * UNPROTECTED - No authentication required
   *
   * @returns Promise resolving to general ingestion configuration
   */
  async getConfig(): Promise<EnhancedApiResponse<IngestionConfig>> {
    return this.client.get<IngestionConfig>(
      API_ENDPOINTS.GET_INGESTION_CONFIG,
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.STANDARD,
        cacheable: false, // Config should not be cached for security
      },
    );
  }

  /**
   * Save AI provider configuration
   * UNPROTECTED - No authentication required
   *
   * @param config The Ingestion configuration to save
   * @returns Promise resolving to save operation result
   */
  async saveConfig(
    config: IngestionConfig,
  ): Promise<EnhancedApiResponse<{ success: boolean; message: string }>> {
    return this.client.post<{ success: boolean; message: string }>(
      API_ENDPOINTS.GET_INGESTION_CONFIG,
      config,
      {
        timeout: API_TIMEOUTS.CONFIG, // Longer timeout for config operations
        retries: API_RETRIES.LIMITED, // Limited retries for config changes
        cacheable: false, // Never cache config operations
      },
    );
  }

  /**
   * Validate JSON data structure for ingestion
   * UNPROTECTED - Validation is a utility operation
   *
   * @param data The JSON data to validate
   * @returns Promise resolving to validation result
   */
  async validateData(
    data: ValidationRequest,
  ): Promise<EnhancedApiResponse<ValidationResponse>> {
    return this.client.post<ValidationResponse>(
      API_ENDPOINTS.VALIDATE_JSON,
      data,
      {
        requiresAuth: false, // Validation is a utility operation
        timeout: API_TIMEOUTS.MUTATION, // Longer timeout for AI analysis
        retries: API_RETRIES.STANDARD,
        cacheable: false, // Validation results should not be cached
      },
    );
  }

  /**
   * Process data ingestion with AI analysis
   * UNPROTECTED - UI does not require authentication per project preference
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
    } = {},
  ): Promise<EnhancedApiResponse<ProcessIngestionResponse>> {
    // Generate a UUID for progress tracking
    const progressId = crypto.randomUUID();

    const request: ProcessIngestionRequest = {
      data,
      auto_execute: options.autoExecute ?? true,
      trust_distance: options.trustDistance ?? 0,
      pub_key: options.pubKey ?? "default",
      progress_id: progressId,
    };

    // Validate request before sending
    const validation = this.validateIngestionRequest(request);
    if (!validation.isValid) {
      throw new Error(
        `Invalid ingestion request: ${validation.errors.join(", ")}`,
      );
    }

    return this.client.post<ProcessIngestionResponse>(
      API_ENDPOINTS.PROCESS_JSON,
      request,
      {
        timeout: API_TIMEOUTS.AI_PROCESSING, // Extended timeout for AI processing (60 seconds)
        retries: API_RETRIES.LIMITED, // Limited retries for processing operations
        cacheable: false, // Processing results should not be cached
      },
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
    if (!request.data || typeof request.data !== "object") {
      errors.push("Data must be a valid object");
    } else if (Object.keys(request.data).length === 0) {
      errors.push("Data cannot be empty");
    }

    // Validate trust distance
    if (
      typeof request.trust_distance !== "number" ||
      request.trust_distance < 0
    ) {
      errors.push("Trust distance must be a non-negative number");
    } else if (request.trust_distance > 10) {
      warnings.push("Trust distance is unusually high");
    }

    // Validate public key
    if (!request.pub_key || request.pub_key.trim().length === 0) {
      errors.push("Public key is required");
    }

    // Validate auto_execute flag
    if (typeof request.auto_execute !== "boolean") {
      errors.push("Auto execute must be a boolean value");
    }

    return {
      isValid: errors.length === 0,
      errors,
      warnings,
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
      progressId?: string;
    } = {},
  ): ProcessIngestionRequest {
    return {
      data: { ...data }, // Create a copy
      auto_execute: options.autoExecute ?? true,
      trust_distance: options.trustDistance ?? 0,
      pub_key: options.pubKey ?? "default",
      progress_id: options.progressId ?? crypto.randomUUID(),
    };
  }

  /**
   * Scan a folder for files to ingest
   */
  async smartFolderScan(
    folderPath: string,
    maxDepth = 10,
    maxFiles = 100,
  ): Promise<EnhancedApiResponse<{ success: boolean; progress_id: string }>> {
    return this.client.post<{ success: boolean; progress_id: string }>(
      "/ingestion/smart-folder/scan",
      {
        folder_path: folderPath,
        max_depth: maxDepth,
        max_files: maxFiles,
      },
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Get the completed scan result by progress ID
   */
  async getScanResult(
    progressId: string,
  ): Promise<EnhancedApiResponse<SmartFolderScanResponse>> {
    return this.client.get<SmartFolderScanResponse>(
      `/ingestion/smart-folder/scan/${progressId}`,
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Ingest selected files from a smart folder scan
   */
  async smartFolderIngest(
    folderPath: string,
    files: string[],
    autoExecute = true,
    spendLimit?: number,
    fileCosts?: number[],
    forceReingest = false,
  ): Promise<EnhancedApiResponse<SmartFolderIngestResponse>> {
    return this.client.post<SmartFolderIngestResponse>(
      "/ingestion/smart-folder/ingest",
      {
        folder_path: folderPath,
        files_to_ingest: files,
        auto_execute: autoExecute,
        spend_limit: spendLimit ?? null,
        file_costs: fileCosts ?? null,
        force_reingest: forceReingest,
      },
      {
        timeout: API_TIMEOUTS.AI_PROCESSING,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Get batch status (cost, progress, pause state)
   */
  async getBatchStatus(
    batchId: string,
  ): Promise<EnhancedApiResponse<BatchStatusResponse>> {
    return this.client.get<BatchStatusResponse>(
      `/ingestion/batch/${batchId}`,
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Resume a paused batch with a new spend limit
   */
  async resumeBatch(
    batchId: string,
    newSpendLimit: number,
  ): Promise<EnhancedApiResponse<BatchStatusResponse>> {
    return this.client.post<BatchStatusResponse>(
      "/ingestion/smart-folder/resume",
      { batch_id: batchId, new_spend_limit: newSpendLimit },
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Cancel a running or paused batch
   */
  async cancelBatch(
    batchId: string,
  ): Promise<EnhancedApiResponse<BatchStatusResponse>> {
    return this.client.post<BatchStatusResponse>(
      "/ingestion/smart-folder/cancel",
      { batch_id: batchId },
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Complete a partial filesystem path with matching directories
   */
  async completePath(
    partialPath: string,
  ): Promise<EnhancedApiResponse<{ completions: string[] }>> {
    return this.client.post<{ completions: string[] }>(
      "/system/complete-path",
      { partial_path: partialPath },
      {
        timeout: API_TIMEOUTS.QUICK,
        retries: API_RETRIES.NONE,
        cacheable: false,
      },
    );
  }

  /**
   * Upload a file for AI-powered ingestion
   * UNPROTECTED - No authentication required per project preference
   *
   * @param file The file to upload
   * @param options Upload options (progressId, autoExecute, trustDistance, pubKey)
   * @returns Promise resolving to upload/processing result
   */
  async uploadFile(
    file: File,
    options: {
      progressId?: string;
      autoExecute?: boolean;
      trustDistance?: number;
      pubKey?: string;
    } = {},
  ): Promise<EnhancedApiResponse<FileUploadResponse>> {
    const formData = new FormData();
    formData.append('progress_id', options.progressId ?? crypto.randomUUID());
    formData.append('file', file);
    formData.append('autoExecute', String(options.autoExecute ?? true));
    formData.append('trustDistance', String(options.trustDistance ?? 0));
    formData.append('pubKey', options.pubKey ?? 'default');

    return this.client.post<FileUploadResponse>(
      API_ENDPOINTS.INGESTION_UPLOAD,
      formData,
      {
        headers: { 'Content-Type': CONTENT_TYPES.FORM_DATA },
        timeout: API_TIMEOUTS.AI_PROCESSING,
        retries: API_RETRIES.LIMITED,
        cacheable: false,
      },
    );
  }

  /**
   * Get API metrics for ingestion operations
   */
  getMetrics() {
    return this.client
      .getMetrics()
      .filter((metric) => metric.url.includes("/ingestion"));
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
export function createIngestionClient(
  client?: ApiClient,
): UnifiedIngestionClient {
  return new UnifiedIngestionClient(client);
}

export default ingestionClient;
