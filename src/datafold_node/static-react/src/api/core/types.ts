/**
 * Core API Types for Unified Client
 * Standardized interfaces for all API operations
 */

import type { ApiResponse } from '../../types/api';

// Re-export existing ApiResponse for backward compatibility
export type { ApiResponse };

// API Error Class Interface
export interface ApiErrorInterface extends Error {
  status: number;
  response?: any;
  isNetworkError: boolean;
  isTimeoutError: boolean;
  isRetryable: boolean;
  requestId?: string;
  timestamp: number;
  code?: string;
  details?: Record<string, any>;
  toUserMessage(): string;
  toJSON(): any;
}

// Enhanced API Response with metadata
export interface EnhancedApiResponse<T = any> extends ApiResponse<T> {
  status: number;
  headers?: Record<string, string>;
  meta?: {
    requestId?: string;
    timestamp: number;
    cached?: boolean;
    fromCache?: boolean;
  };
}

// Request Configuration Options
export interface RequestOptions {
  requiresAuth?: boolean;
  timeout?: number;
  retries?: number;
  validateSchema?: boolean | SchemaValidationOptions;
  cacheable?: boolean;
  cacheKey?: string;
  cacheTtl?: number;
  requestId?: string;
  abortSignal?: AbortSignal;
  priority?: 'low' | 'normal' | 'high';
}

// API Client Configuration
export interface ApiClientConfig {
  baseUrl?: string;
  timeout?: number;
  retryAttempts?: number;
  retryDelay?: number;
  defaultHeaders?: Record<string, string>;
  enableCache?: boolean;
  enableLogging?: boolean;
  enableMetrics?: boolean;
}

// HTTP Method Types
export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH';

// Request Interceptor Function
export type RequestInterceptor = (config: RequestConfig) => RequestConfig | Promise<RequestConfig>;

// Response Interceptor Function
export type ResponseInterceptor<T = any> = (response: EnhancedApiResponse<T>) => EnhancedApiResponse<T> | Promise<EnhancedApiResponse<T>>;

// Error Interceptor Function
export type ErrorInterceptor = (error: ApiErrorInterface) => ApiErrorInterface | Promise<ApiErrorInterface>;

// Internal Request Configuration
export interface RequestConfig {
  url: string;
  method: HttpMethod;
  headers: Record<string, string>;
  body?: any;
  timeout: number;
  retries: number;
  validateSchema: boolean;
  requiresAuth: boolean;
  abortSignal?: AbortSignal;
  metadata: {
    requestId: string;
    timestamp: number;
    priority: 'low' | 'normal' | 'high';
  };
}

// Cache Entry Interface
export interface CacheEntry<T = any> {
  data: T;
  timestamp: number;
  ttl: number;
  key: string;
}

// Retry Configuration
export interface RetryConfig {
  attempts: number;
  delay: number;
  backoffMultiplier: number;
  maxDelay: number;
  retryableStatusCodes: number[];
}

// Schema Validation Interface
export interface SchemaValidationOptions {
  requiresApproved?: boolean;
  operation?: 'read' | 'write' | 'approve' | 'block';
  schemaName?: string;
}

// Request Metrics Interface
export interface RequestMetrics {
  requestId: string;
  url: string;
  method: HttpMethod;
  startTime: number;
  endTime?: number;
  duration?: number;
  status?: number;
  cached?: boolean;
  retryCount?: number;
  error?: string;
}

// Batch Request Interface
export interface BatchRequest {
  id: string;
  method: HttpMethod;
  url: string;
  body?: any;
  options?: RequestOptions;
}

// Batch Response Interface
export interface BatchResponse<T = any> {
  id: string;
  success: boolean;
  data?: T;
  error?: string;
  status: number;
}

// API Error Interface
export interface ApiErrorDetails {
  message: string;
  status: number;
  code?: string;
  details?: Record<string, any>;
  response?: any;
  isNetworkError: boolean;
  isTimeoutError: boolean;
  isRetryable: boolean;
  requestId?: string;
  timestamp: number;
}

// Schema State Validation Result
export interface SchemaStateValidation {
  isValid: boolean;
  schemaName: string;
  currentState: string;
  operation: string;
  error?: string;
}

// Client Instance Interface
export interface ApiClientInstance {
  get<T>(endpoint: string, options?: RequestOptions): Promise<EnhancedApiResponse<T>>;
  post<T>(endpoint: string, data?: any, options?: RequestOptions): Promise<EnhancedApiResponse<T>>;
  put<T>(endpoint: string, data?: any, options?: RequestOptions): Promise<EnhancedApiResponse<T>>;
  delete<T>(endpoint: string, options?: RequestOptions): Promise<EnhancedApiResponse<T>>;
  patch<T>(endpoint: string, data?: any, options?: RequestOptions): Promise<EnhancedApiResponse<T>>;
  batch<T>(requests: BatchRequest[]): Promise<BatchResponse<T>[]>;
  
  // Interceptor management
  addRequestInterceptor(interceptor: RequestInterceptor): void;
  addResponseInterceptor<T>(interceptor: ResponseInterceptor<T>): void;
  addErrorInterceptor(interceptor: ErrorInterceptor): void;
  
  // Cache management
  clearCache(): void;
  getCacheStats(): { size: number; hitRate: number };
  
  // Metrics
  getMetrics(): RequestMetrics[];
  clearMetrics(): void;
}

// Domain-specific client interfaces for type safety
export interface SchemaApiClient {
  getSchemas(): Promise<EnhancedApiResponse<any[]>>;
  getSchema(name: string): Promise<EnhancedApiResponse<any>>;
  getSchemasByState(state: string): Promise<EnhancedApiResponse<any>>;
  getAllSchemasWithState(): Promise<EnhancedApiResponse<any>>;
  getSchemaStatus(): Promise<EnhancedApiResponse<any>>;
  approveSchema(name: string): Promise<EnhancedApiResponse<void>>;
  blockSchema(name: string): Promise<EnhancedApiResponse<void>>;
}

export interface MutationApiClient {
  executeMutation(signedMessage: any): Promise<EnhancedApiResponse<any>>;
  executeQuery(signedMessage: any): Promise<EnhancedApiResponse<any>>;
  validateMutation(mutation: any): Promise<EnhancedApiResponse<any>>;
}

export interface SecurityApiClient {
  verifyMessage(signedMessage: any): Promise<EnhancedApiResponse<any>>;
  registerPublicKey(request: any): Promise<EnhancedApiResponse<any>>;
  getSystemPublicKey(): Promise<EnhancedApiResponse<any>>;
}
