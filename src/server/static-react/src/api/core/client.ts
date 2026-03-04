/**
 * Unified API Client
 * Standardized HTTP client with authentication, caching, retry logic, and error handling
 */

import {
  API_REQUEST_TIMEOUT_MS,
  API_RETRY_ATTEMPTS,
  API_RETRY_DELAY_MS,
  API_CONFIG,
  CONTENT_TYPES,
  REQUEST_HEADERS,
  RETRY_CONFIG,
  CACHE_CONFIG,
} from "../../constants/api";
import { BROWSER_CONFIG } from "../../constants/config";

import {
  ApiError,
  NetworkError,
  TimeoutError,
  ErrorFactory,
  isRetryableError,
} from "./errors";

import { createSignatureInterceptor } from "../interceptors/signatureInterceptor";

import type {
  ApiClientConfig,
  RequestOptions,
  EnhancedApiResponse,
  RequestConfig,
  HttpMethod,
  RequestInterceptor,
  ResponseInterceptor,
  CacheEntry,
  RequestMetrics,
  BatchRequest,
  BatchResponse,
  ApiClientInstance,
} from "./types";

// Define ErrorInterceptor locally to use concrete ApiError class
type ErrorInterceptor = (error: ApiError) => ApiError | Promise<ApiError>;

/**
 * In-memory cache implementation
 */
class ApiCache {
  private cache = new Map<string, CacheEntry>();
  private readonly maxSize: number;
  private hits = 0;
  private misses = 0;

  constructor(maxSize: number = CACHE_CONFIG.MAX_CACHE_SIZE) {
    this.maxSize = maxSize;
  }

  get<T>(key: string): T | null {
    const entry = this.cache.get(key);
    if (!entry) {
      this.misses++;
      return null;
    }

    // Check if entry has expired
    if (Date.now() > entry.timestamp + entry.ttl) {
      this.cache.delete(key);
      this.misses++;
      return null;
    }

    this.hits++;
    return entry.data as T;
  }

  set<T>(
    key: string,
    data: T,
    ttl: number = CACHE_CONFIG.DEFAULT_TTL_MS,
  ): void {
    // Implement LRU eviction if cache is full
    if (this.cache.size >= this.maxSize) {
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }

    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      ttl,
      key,
    });
  }

  clear(): void {
    this.cache.clear();
  }

  size(): number {
    return this.cache.size;
  }

  getHitRate(): number {
    const total = this.hits + this.misses;
    return total > 0 ? this.hits / total : 0;
  }
}

/**
 * Request queue for managing concurrent requests
 */
class RequestQueue {
  private queue = new Map<string, Promise<unknown>>();

  /**
   * Get or create a request promise to prevent duplicate requests
   */
  getOrCreate<T>(key: string, requestFn: () => Promise<T>): Promise<T> {
    if (this.queue.has(key)) {
      return this.queue.get(key)!;
    }

    const promise = requestFn().finally(() => {
      this.queue.delete(key);
    });

    this.queue.set(key, promise);
    return promise;
  }

  clear(): void {
    this.queue.clear();
  }
}

/**
 * Main API Client Class
 */
export class ApiClient implements ApiClientInstance {
  private readonly config: Required<ApiClientConfig>;
  private readonly cache: ApiCache;
  private readonly requestQueue: RequestQueue;
  private readonly requestInterceptors: RequestInterceptor[] = [];
  private readonly responseInterceptors: ResponseInterceptor[] = [];
  private readonly errorInterceptors: ErrorInterceptor[] = [];
  private readonly metrics: RequestMetrics[] = [];

  constructor(config: ApiClientConfig = {}) {
    this.config = {
      baseUrl: config.baseUrl || API_CONFIG.BASE_URL,
      timeout: config.timeout || API_REQUEST_TIMEOUT_MS,
      retryAttempts: config.retryAttempts || API_RETRY_ATTEMPTS,
      retryDelay: config.retryDelay || API_RETRY_DELAY_MS,
      defaultHeaders: config.defaultHeaders || {},
      enableCache: config.enableCache !== false,
      enableLogging: config.enableLogging !== false,
      enableMetrics: config.enableMetrics !== false,
    };

    this.cache = new ApiCache();
    this.requestQueue = new RequestQueue();
  }

  /**
   * HTTP GET method
   */
  async get<T>(
    endpoint: string,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    return this.request<T>("GET", endpoint, undefined, options);
  }

  /**
   * HTTP POST method
   */
  async post<T>(
    endpoint: string,
    data?: unknown,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    return this.request<T>("POST", endpoint, data, options);
  }

  /**
   * HTTP PUT method
   */
  async put<T>(
    endpoint: string,
    data?: unknown,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    return this.request<T>("PUT", endpoint, data, options);
  }

  /**
   * HTTP DELETE method
   */
  async delete<T>(
    endpoint: string,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    return this.request<T>("DELETE", endpoint, undefined, options);
  }

  /**
   * HTTP PATCH method
   */
  async patch<T>(
    endpoint: string,
    data?: unknown,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    return this.request<T>("PATCH", endpoint, data, options);
  }

  /**
   * Batch request processing
   */
  async batch<T>(requests: BatchRequest[]): Promise<BatchResponse<T>[]> {
    if (requests.length > CACHE_CONFIG.MAX_CACHE_SIZE) {
      throw new ApiError(
        `Batch size exceeds limit of ${CACHE_CONFIG.MAX_CACHE_SIZE}`,
      );
    }

    const promises = requests.map(
      async (request): Promise<BatchResponse<T>> => {
        try {
          const response = await this.request<T>(
            request.method,
            request.url,
            request.body,
            request.options,
          );

          return {
            id: request.id,
            success: response.success,
            data: response.data,
            status: response.status,
          };
        } catch (error: unknown) {
          const apiError =
            error instanceof ApiError ? error : new ApiError(error instanceof Error ? error.message : String(error));
          return {
            id: request.id,
            success: false,
            error: apiError.message,
            status: apiError.status,
          };
        }
      },
    );

    return Promise.all(promises);
  }

  /**
   * Core request method with all functionality
   */
  private async request<T>(
    method: HttpMethod,
    endpoint: string,
    data?: unknown,
    options: RequestOptions = {},
  ): Promise<EnhancedApiResponse<T>> {
    const requestId = options.requestId || this.generateRequestId();
    const startTime = Date.now();

    let config: RequestConfig = {
      url: this.buildUrl(endpoint),
      method,
      headers: { ...this.config.defaultHeaders },
      body: data,
      timeout: options.timeout || this.config.timeout,
      retries:
        options.retries !== undefined
          ? options.retries
          : this.config.retryAttempts,
      validateSchema: !!options.validateSchema,
      requiresAuth: false,
      abortSignal: options.abortSignal,
      metadata: {
        requestId,
        timestamp: startTime,
        priority: options.priority || "normal",
      },
    };

    try {
      // Apply request interceptors
      for (const interceptor of this.requestInterceptors) {
        config = await interceptor(config);
      }

      // Check cache for GET requests
      if (
        method === "GET" &&
        this.config.enableCache &&
        options.cacheable !== false
      ) {
        const cacheKey = this.generateCacheKey(config.url, config.headers);
        const cachedResponse = this.cache.get<EnhancedApiResponse<T>>(cacheKey);

        if (cachedResponse) {
          return {
            ...cachedResponse,
            meta: {
              ...cachedResponse.meta,
              cached: true,
              fromCache: true,
              requestId,
              timestamp: cachedResponse.meta?.timestamp || Date.now(),
            },
          };
        }
      }

      // Deduplicate concurrent requests
      const dedupeKey = `${method}:${config.url}:${JSON.stringify(data)}`;
      const response = await this.requestQueue.getOrCreate(dedupeKey, () =>
        this.executeRequest<T>(config),
      );

      // Cache successful GET responses
      if (
        method === "GET" &&
        this.config.enableCache &&
        options.cacheable !== false &&
        response.success
      ) {
        const cacheKey = this.generateCacheKey(config.url, config.headers);
        const cacheTtl = options.cacheTtl || CACHE_CONFIG.DEFAULT_TTL_MS;
        this.cache.set(cacheKey, response, cacheTtl);
      }

      // Apply response interceptors
      let finalResponse = response;
      for (const interceptor of this.responseInterceptors) {
        finalResponse = (await interceptor(
          finalResponse,
        )) as EnhancedApiResponse<T>;
      }

      // Record metrics
      if (this.config.enableMetrics) {
        this.recordMetrics({
          requestId,
          url: config.url,
          method,
          startTime,
          endTime: Date.now(),
          duration: Date.now() - startTime,
          status: response.status,
          cached: response.meta?.cached || false,
        });
      }

      return finalResponse;
    } catch (error: unknown) {
      let apiError =
        error instanceof ApiError
          ? error
          : ErrorFactory.fromNetworkError(error instanceof Error ? error : new Error(String(error)), requestId);

      // Apply error interceptors
      for (const interceptor of this.errorInterceptors) {
        apiError = await interceptor(apiError);
      }

      // Record error metrics
      if (this.config.enableMetrics) {
        this.recordMetrics({
          requestId,
          url: config.url,
          method,
          startTime,
          endTime: Date.now(),
          duration: Date.now() - startTime,
          error: apiError.message,
        });
      }

      throw apiError;
    }
  }

  /**
   * Execute the actual HTTP request with retry logic
   */
  private async executeRequest<T>(
    config: RequestConfig,
  ): Promise<EnhancedApiResponse<T>> {
    let lastError: ApiError;

    for (let attempt = 0; attempt <= config.retries; attempt++) {
      try {
        return await this.performRequest<T>(config);
      } catch (error: unknown) {
        lastError =
          error instanceof ApiError
            ? error
            : ErrorFactory.fromNetworkError(error instanceof Error ? error : new Error(String(error)), config.metadata.requestId);

        // Don't retry on final attempt or non-retryable errors
        if (attempt === config.retries || !isRetryableError(lastError)) {
          break;
        }

        // Calculate exponential backoff delay
        const delay = Math.min(
          this.config.retryDelay *
            Math.pow(RETRY_CONFIG.EXPONENTIAL_BACKOFF_MULTIPLIER, attempt),
          RETRY_CONFIG.MAX_RETRY_DELAY_MS,
        );

        await this.sleep(delay);
      }
    }

    throw lastError!;
  }

  /**
   * Perform the actual HTTP request
   */
  private async performRequest<T>(
    config: RequestConfig,
  ): Promise<EnhancedApiResponse<T>> {
    // Set up timeout
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), config.timeout);

    try {
      // Prepare headers
      const headers = { ...config.headers };

      // No authentication: UI does not require or send auth headers

      // Set content type for requests with body
      if (config.body && !headers[REQUEST_HEADERS.CONTENT_TYPE]) {
        headers[REQUEST_HEADERS.CONTENT_TYPE] = CONTENT_TYPES.JSON;
      }

      // Add request metadata headers
      headers[REQUEST_HEADERS.REQUEST_ID] = config.metadata.requestId;

      // Add User ID header (Strict User Isolation)
      // Send both x-user-hash (for exemem cloud) and x-user-id (for legacy)
      if (typeof window !== "undefined") {
        const userHash = localStorage.getItem(BROWSER_CONFIG.STORAGE_KEYS.USER_HASH);
        if (userHash) {
          headers["x-user-hash"] = userHash; // Primary: for exemem cloud
          headers["x-user-id"] = userHash; // Fallback: for standalone
        }
      }

      // Prepare fetch options
      const fetchOptions: RequestInit = {
        method: config.method,
        headers,
        signal: config.abortSignal || controller.signal,
      };

      // Add body for non-GET requests
      if (config.body && config.method !== "GET") {
        fetchOptions.body = this.serializeBody(
          config.body,
          headers[REQUEST_HEADERS.CONTENT_TYPE],
        );
        // When body is FormData, delete Content-Type so the browser
        // auto-sets it with the correct multipart boundary
        if (fetchOptions.body instanceof FormData) {
          delete headers[REQUEST_HEADERS.CONTENT_TYPE];
        }
      }

      // Perform the request
      // eslint-disable-next-line no-restricted-globals -- Core HTTP client layer legitimately uses fetch()
      const response = await fetch(config.url, fetchOptions);

      clearTimeout(timeoutId);

      // Handle response
      return await this.handleResponse<T>(response, config.metadata.requestId);
    } catch (error: unknown) {
      clearTimeout(timeoutId);

      const err = error instanceof Error ? error : new Error(String(error));
      if (err.name === "AbortError") {
        throw ErrorFactory.fromTimeout(
          config.timeout,
          config.metadata.requestId,
        );
      }

      throw ErrorFactory.fromNetworkError(err, config.metadata.requestId);
    }
  }

  /**
   * Handle HTTP response and convert to standardized format
   */
  private async handleResponse<T>(
    response: Response,
    requestId: string,
  ): Promise<EnhancedApiResponse<T>> {
    if (!response.ok) {
      throw await ErrorFactory.fromResponse(response, requestId);
    }

    let data: T;
    const contentType = response.headers.get("content-type");

    try {
      if (contentType?.includes("application/json")) {
        data = await response.json();
      } else {
        data = (await response.text()) as T;
      }
    } catch {
      throw new ApiError("Failed to parse response", response.status, {
        requestId,
      });
    }

    return {
      success: true,
      data,
      status: response.status,
      headers: this.extractHeaders(response.headers),
      meta: {
        requestId,
        timestamp: Date.now(),
        cached: false,
        fromCache: false,
      },
    };
  }

  /**
   * Serialize request body based on content type
   */
  private serializeBody(body: unknown, contentType: string): string | FormData {
    if (contentType === CONTENT_TYPES.JSON) {
      return JSON.stringify(body);
    }
    if (contentType === CONTENT_TYPES.FORM_DATA) {
      return body as FormData; // Assume FormData is passed directly
    }
    return String(body);
  }

  /**
   * Extract response headers as plain object
   */
  private extractHeaders(headers: Headers): Record<string, string> {
    const result: Record<string, string> = {};
    headers.forEach((value, key) => {
      result[key] = value;
    });
    return result;
  }

  /**
   * Generate unique request ID
   */
  private generateRequestId(): string {
    return `req_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
  }

  /**
   * Generate cache key for request
   */
  private generateCacheKey(
    url: string,
    headers: Record<string, string>,
  ): string {
    const relevantHeaders = Object.keys(headers)
      .filter((key) => !key.startsWith("X-Request"))
      .sort()
      .map((key) => `${key}:${headers[key]}`)
      .join(";");

    return `${url}|${relevantHeaders}`;
  }

  /**
   * Build full URL from endpoint
   */
  private buildUrl(endpoint: string): string {
    if (endpoint.startsWith("http")) {
      return endpoint;
    }
    return `${this.config.baseUrl}${endpoint.startsWith("/") ? "" : "/"}${endpoint}`;
  }

  /**
   * Sleep utility for retry delays
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  /**
   * Record request metrics
   */
  private recordMetrics(metrics: RequestMetrics): void {
    this.metrics.push(metrics);

    // Keep only last 1000 metrics to prevent memory leaks
    if (this.metrics.length > 1000) {
      this.metrics.splice(0, this.metrics.length - 1000);
    }
  }

  // Interceptor management methods
  addRequestInterceptor(interceptor: RequestInterceptor): void {
    this.requestInterceptors.push(interceptor);
  }

  addResponseInterceptor<T>(interceptor: ResponseInterceptor<T>): void {
    this.responseInterceptors.push(interceptor as ResponseInterceptor);
  }

  addErrorInterceptor(interceptor: ErrorInterceptor): void {
    this.errorInterceptors.push(interceptor);
  }

  // Cache management methods
  clearCache(): void {
    this.cache.clear();
  }

  getCacheStats(): { size: number; hitRate: number } {
    return {
      size: this.cache.size(),
      hitRate: this.cache.getHitRate(),
    };
  }

  // Metrics methods
  getMetrics(): RequestMetrics[] {
    return [...this.metrics];
  }

  clearMetrics(): void {
    this.metrics.length = 0;
  }
}

// Shared singleton client instance with default configuration
// All domain clients should use this instance to share cache and metrics
let sharedClient: ApiClient | null = null;

export function getSharedClient(): ApiClient {
  if (!sharedClient) {
    sharedClient = new ApiClient({
      enableCache: true,
      enableLogging: true,
      enableMetrics: true,
    });
    // Register signature interceptor to sign write requests
    sharedClient.addRequestInterceptor(createSignatureInterceptor());
  }
  return sharedClient;
}

// Legacy export - use getSharedClient() instead
export const defaultApiClient = getSharedClient();

// Factory function for creating custom clients with non-default config
// For most use cases, use getSharedClient() to benefit from shared caching
export function createApiClient(config?: ApiClientConfig): ApiClient {
  // If no config or empty config, return shared instance
  if (!config || Object.keys(config).length === 0) {
    return getSharedClient();
  }
  // Custom config gets a new instance
  return new ApiClient(config);
}
