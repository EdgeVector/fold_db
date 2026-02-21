/**
 * Unified API Error Classes
 * Standardized error handling for all API operations
 */

import { 
  HTTP_STATUS_CODES, 
  ERROR_MESSAGES, 
  RETRY_CONFIG 
} from '../../constants/api';

/**
 * Base API Error class with enhanced functionality
 */
export class ApiError extends Error {
  public readonly status: number;
  public readonly response?: Response | Record<string, unknown>;
  public readonly isNetworkError: boolean;
  public readonly isTimeoutError: boolean;
  public readonly isRetryable: boolean;
  public readonly requestId?: string;
  public readonly timestamp: number;
  public readonly code?: string;
  public readonly details?: Record<string, unknown>;

  constructor(
    message: string,
    status: number = 0,
    options: {
      response?: Response | Record<string, unknown>;
      isNetworkError?: boolean;
      isTimeoutError?: boolean;
      requestId?: string;
      code?: string;
      details?: Record<string, unknown>;
    } = {}
  ) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
    this.response = options.response;
    this.isNetworkError = options.isNetworkError || false;
    this.isTimeoutError = options.isTimeoutError || false;
    this.isRetryable = this.determineRetryability(status, options.isNetworkError, options.isTimeoutError);
    this.requestId = options.requestId;
    this.timestamp = Date.now();
    this.code = options.code;
    this.details = options.details;

    // Maintain proper prototype chain
    Object.setPrototypeOf(this, ApiError.prototype);
  }

  /**
   * Determines if an error is retryable based on status code and error type
   */
  private determineRetryability(status: number, isNetworkError?: boolean, isTimeoutError?: boolean): boolean {
    if (isNetworkError || isTimeoutError) {
      return true;
    }
    return (RETRY_CONFIG.RETRYABLE_STATUS_CODES as readonly number[]).includes(status);
  }

  /**
   * Convert error to a user-friendly message
   */
  public toUserMessage(): string {
    if (this.isNetworkError) {
      return ERROR_MESSAGES.NETWORK_ERROR;
    }
    
    if (this.isTimeoutError) {
      return ERROR_MESSAGES.TIMEOUT_ERROR;
    }

    switch (this.status) {
      case HTTP_STATUS_CODES.UNAUTHORIZED:
        return ERROR_MESSAGES.AUTHENTICATION_ERROR;
      case HTTP_STATUS_CODES.FORBIDDEN:
        return ERROR_MESSAGES.PERMISSION_ERROR;
      case HTTP_STATUS_CODES.NOT_FOUND:
        return ERROR_MESSAGES.NOT_FOUND_ERROR;
      case HTTP_STATUS_CODES.BAD_REQUEST:
        return ERROR_MESSAGES.VALIDATION_ERROR;
      case HTTP_STATUS_CODES.INTERNAL_SERVER_ERROR:
      case HTTP_STATUS_CODES.BAD_GATEWAY:
      case HTTP_STATUS_CODES.SERVICE_UNAVAILABLE:
        return ERROR_MESSAGES.SERVER_ERROR;
      case 429:
        return ERROR_MESSAGES.RATE_LIMIT_ERROR;
      default:
        return this.message || ERROR_MESSAGES.SERVER_ERROR;
    }
  }

  /**
   * Serialize error for logging
   */
  public toJSON() {
    return {
      name: this.name,
      message: this.message,
      status: this.status,
      isNetworkError: this.isNetworkError,
      isTimeoutError: this.isTimeoutError,
      isRetryable: this.isRetryable,
      requestId: this.requestId,
      timestamp: this.timestamp,
      code: this.code,
      details: this.details,
      stack: this.stack
    };
  }
}

/**
 * Authentication-specific error
 */
export class AuthenticationError extends ApiError {
  constructor(message: string = ERROR_MESSAGES.AUTHENTICATION_ERROR, requestId?: string) {
    super(message, HTTP_STATUS_CODES.UNAUTHORIZED, {
      code: 'AUTH_ERROR',
      requestId
    });
    this.name = 'AuthenticationError';
    Object.setPrototypeOf(this, AuthenticationError.prototype);
  }
}

/**
 * Schema state validation error (SCHEMA-002 compliance)
 */
export class SchemaStateError extends ApiError {
  public readonly schemaName: string;
  public readonly currentState: string;
  public readonly operation: string;

  constructor(
    schemaName: string,
    currentState: string,
    operation: string,
    message: string = ERROR_MESSAGES.SCHEMA_STATE_ERROR
  ) {
    super(message, HTTP_STATUS_CODES.FORBIDDEN, {
      code: 'SCHEMA_STATE_ERROR',
      details: { schemaName, currentState, operation }
    });
    this.name = 'SchemaStateError';
    this.schemaName = schemaName;
    this.currentState = currentState;
    this.operation = operation;
    Object.setPrototypeOf(this, SchemaStateError.prototype);
  }
}

/**
 * Network connectivity error
 */
export class NetworkError extends ApiError {
  constructor(message: string = ERROR_MESSAGES.NETWORK_ERROR, requestId?: string) {
    super(message, 0, {
      isNetworkError: true,
      code: 'NETWORK_ERROR',
      requestId
    });
    this.name = 'NetworkError';
    Object.setPrototypeOf(this, NetworkError.prototype);
  }
}

/**
 * Request timeout error
 */
export class TimeoutError extends ApiError {
  public readonly timeoutMs: number;

  constructor(timeoutMs: number, requestId?: string) {
    super(`Request timed out after ${timeoutMs}ms`, 408, {
      isTimeoutError: true,
      code: 'TIMEOUT_ERROR',
      requestId,
      details: { timeoutMs }
    });
    this.name = 'TimeoutError';
    this.timeoutMs = timeoutMs;
    Object.setPrototypeOf(this, TimeoutError.prototype);
  }
}

/**
 * Validation error for request data
 */
export class ValidationError extends ApiError {
  public readonly validationErrors: Record<string, string[]>;

  constructor(validationErrors: Record<string, string[]>, requestId?: string) {
    const message = 'Request validation failed';
    super(message, HTTP_STATUS_CODES.BAD_REQUEST, {
      code: 'VALIDATION_ERROR',
      requestId,
      details: { validationErrors }
    });
    this.name = 'ValidationError';
    this.validationErrors = validationErrors;
    Object.setPrototypeOf(this, ValidationError.prototype);
  }
}

/**
 * Rate limiting error
 */
export class RateLimitError extends ApiError {
  public readonly retryAfter?: number;

  constructor(retryAfter?: number, requestId?: string) {
    const message = retryAfter 
      ? `Rate limit exceeded. Retry after ${retryAfter} seconds.`
      : ERROR_MESSAGES.RATE_LIMIT_ERROR;
    
    super(message, 429, {
      code: 'RATE_LIMIT_ERROR',
      requestId,
      details: { retryAfter }
    });
    this.name = 'RateLimitError';
    this.retryAfter = retryAfter;
    Object.setPrototypeOf(this, RateLimitError.prototype);
  }
}

/**
 * Error factory for creating appropriate error instances
 */
export class ErrorFactory {
  /**
   * Create an ApiError from a fetch response
   */
  static async fromResponse(
    response: Response, 
    requestId?: string
  ): Promise<ApiError> {
    let errorData: Record<string, unknown> = {};
    
    try {
      const text = await response.text();
      if (text) {
        errorData = JSON.parse(text);
      }
    } catch {
      // Ignore JSON parsing errors
    }

    const message = (typeof errorData.error === 'string' ? errorData.error :
                     typeof errorData.message === 'string' ? errorData.message :
                     `HTTP ${response.status}`);
    
    // Check for specific error types
    if (response.status === HTTP_STATUS_CODES.UNAUTHORIZED) {
      return new AuthenticationError(message, requestId || '');
    }
    
    if (response.status === 429) {
      const retryAfter = response.headers.get('Retry-After');
      return new RateLimitError(retryAfter ? parseInt(retryAfter) : undefined, requestId);
    }
    
    if (response.status === HTTP_STATUS_CODES.BAD_REQUEST && errorData.validationErrors) {
      return new ValidationError(errorData.validationErrors as Record<string, string[]>, requestId || '');
    }

    return new ApiError(message, response.status, {
      response: errorData,
      requestId,
      code: typeof errorData.code === 'string' ? errorData.code : undefined,
      details: typeof errorData.details === 'object' && errorData.details !== null ? errorData.details as Record<string, unknown> : undefined
    });
  }

  /**
   * Create an ApiError from a network error
   */
  static fromNetworkError(error: Error, requestId?: string): NetworkError {
    return new NetworkError(error.message, requestId);
  }

  /**
   * Create an ApiError from a timeout
   */
  static fromTimeout(timeoutMs: number, requestId?: string): TimeoutError {
    return new TimeoutError(timeoutMs, requestId);
  }

  /**
   * Create a schema state error
   */
  static fromSchemaState(
    schemaName: string, 
    currentState: string, 
    operation: string
  ): SchemaStateError {
    return new SchemaStateError(schemaName, currentState, operation);
  }
}

/**
 * Utility functions for error type checking
 */
export function isApiError(error: unknown): error is ApiError {
  return error instanceof ApiError;
}

export function isNetworkError(error: unknown): error is NetworkError {
  return error instanceof NetworkError || (isApiError(error) && error.isNetworkError);
}

export function isTimeoutError(error: unknown): error is TimeoutError {
  return error instanceof TimeoutError || (isApiError(error) && error.isTimeoutError);
}

export function isAuthenticationError(error: unknown): error is AuthenticationError {
  return error instanceof AuthenticationError || 
         (isApiError(error) && error.status === HTTP_STATUS_CODES.UNAUTHORIZED);
}

export function isSchemaStateError(error: unknown): error is SchemaStateError {
  return error instanceof SchemaStateError;
}

export function isRetryableError(error: unknown): boolean {
  return isApiError(error) && error.isRetryable;
}

export function isValidationError(error: unknown): error is ValidationError {
  return error instanceof ValidationError;
}

export function isRateLimitError(error: unknown): error is RateLimitError {
  return error instanceof RateLimitError;
}