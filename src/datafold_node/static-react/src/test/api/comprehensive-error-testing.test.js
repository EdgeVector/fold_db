/**
 * @fileoverview Comprehensive API Error Testing Suite
 * 
 * This file implements comprehensive API error testing for UTC-1-9.
 * Tests cover all major error scenarios including network failures, HTTP status codes,
 * authentication errors, rate limiting, retry mechanisms, and error recovery.
 * 
 * Scope:
 * - Network connectivity errors (timeouts, network failures)
 * - HTTP status code errors (400, 401, 403, 404, 500, 503, etc.)
 * - API response format errors (malformed JSON, missing fields)
 * - Authentication and authorization error scenarios
 * - Rate limiting and quota exceeded errors
 * - Server-side validation errors
 * - Concurrent request handling errors
 * - Retry mechanism testing
 * - Circuit breaker pattern testing
 * 
 * @module comprehensiveApiErrorTesting
 * @since 2.0.0
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { http, HttpResponse, delay } from 'msw';
import { setupServer } from 'msw/node';
import { ApiClient } from '../../api/core/client';
import { 
  ApiError, 
  NetworkError, 
  TimeoutError, 
  AuthenticationError,
  SchemaStateError,
  ValidationError,
  RateLimitError,
  ErrorFactory
} from '../../api/core/errors';
import { 
  HTTP_STATUS_CODES, 
  ERROR_MESSAGES,
  API_TIMEOUTS,
  RETRY_CONFIG
} from '../../constants/api';
import { 
  createMockApiClient,
  setupMockServer,
  withMockHandlers
} from '../mocks/apiMocks';

describe('Comprehensive API Error Testing', () => {
  let apiClient;
  let mockServer;

  beforeEach(() => {
    // Create fresh API client for each test
    apiClient = new ApiClient({
      timeout: 5000,
      retryAttempts: 2,
      enableCache: false,
      enableLogging: false
    });
  });

  describe('Network Connectivity Errors', () => {
    it('should handle network connection failures', async () => {
      const networkErrorHandler = http.get('/api/schemas', () => {
        return HttpResponse.error();
      });

      await withMockHandlers([networkErrorHandler], async () => {
        await expect(apiClient.get('/schemas')).rejects.toThrow(NetworkError);
      });
    });

    it('should handle request timeouts', async () => {
      const timeoutHandler = http.get('/api/schemas', async () => {
        await delay(6000); // Longer than our 5s timeout
        return HttpResponse.json({ success: true });
      });

      await withMockHandlers([timeoutHandler], async () => {
        await expect(apiClient.get('/schemas')).rejects.toThrow(TimeoutError);
      });
    });

    it('should handle DNS resolution failures', async () => {
      const dnsErrorHandler = http.get('/api/schemas', () => {
        throw new TypeError('Failed to fetch'); // Simulates DNS/network error
      });

      await withMockHandlers([dnsErrorHandler], async () => {
        await expect(apiClient.get('/schemas')).rejects.toThrow(NetworkError);
      });
    });

    it('should handle connection refused errors', async () => {
      const connectionRefusedHandler = http.get('/api/schemas', () => {
        return HttpResponse.error();
      });

      await withMockHandlers([connectionRefusedHandler], async () => {
        await expect(apiClient.get('/schemas')).rejects.toThrow(NetworkError);
      });
    });
  });

  describe('HTTP Status Code Errors', () => {
    const statusCodeTests = [
      { status: 400, message: 'Bad Request', expectedError: ApiError },
      { status: 401, message: 'Unauthorized', expectedError: AuthenticationError },
      { status: 403, message: 'Forbidden', expectedError: ApiError },
      { status: 404, message: 'Not Found', expectedError: ApiError },
      { status: 409, message: 'Conflict', expectedError: ApiError },
      { status: 422, message: 'Unprocessable Entity', expectedError: ApiError },
      { status: 429, message: 'Too Many Requests', expectedError: RateLimitError },
      { status: 500, message: 'Internal Server Error', expectedError: ApiError },
      { status: 502, message: 'Bad Gateway', expectedError: ApiError },
      { status: 503, message: 'Service Unavailable', expectedError: ApiError },
      { status: 504, message: 'Gateway Timeout', expectedError: ApiError }
    ];

    statusCodeTests.forEach(({ status, message, expectedError }) => {
      it(`should handle HTTP ${status} - ${message}`, async () => {
        const errorHandler = http.get('/api/schemas', () => {
          return new Response(
            JSON.stringify({
              success: false,
              error: { message: `${status} ${message}` }
            }),
            { 
              status,
              headers: { 'Content-Type': 'application/json' }
            }
          );
        });

        await withMockHandlers([errorHandler], async () => {
          const error = await apiClient.get('/schemas').catch(e => e);
          expect(error).toBeInstanceOf(expectedError);
          expect(error.status).toBe(status);
        });
      });
    });

    it('should handle rate limiting with retry-after header', async () => {
      const rateLimitHandler = http.get('/api/schemas', () => {
        return new Response(
          JSON.stringify({
            success: false,
            error: { message: 'Rate limit exceeded' }
          }),
          { 
            status: 429,
            headers: { 
              'Content-Type': 'application/json',
              'Retry-After': '60'
            }
          }
        );
      });

      await withMockHandlers([rateLimitHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        expect(error).toBeInstanceOf(RateLimitError);
        expect(error.retryAfter).toBe(60);
      });
    });
  });

  describe('API Response Format Errors', () => {
    it('should handle malformed JSON responses', async () => {
      const malformedJsonHandler = http.get('/api/schemas', () => {
        return new Response(
          '{ invalid json syntax',
          { 
            status: 200,
            headers: { 'Content-Type': 'application/json' }
          }
        );
      });

      await withMockHandlers([malformedJsonHandler], async () => {
        await expect(apiClient.get('/schemas')).rejects.toThrow(ApiError);
      });
    });

    it('should handle empty response bodies', async () => {
      const emptyResponseHandler = http.get('/api/schemas', () => {
        return new Response('', { 
          status: 200,
          headers: { 'Content-Type': 'application/json' }
        });
      });

      await withMockHandlers([emptyResponseHandler], async () => {
        const response = await apiClient.get('/schemas');
        expect(response.data).toBe('');
      });
    });

    it('should handle non-JSON content types', async () => {
      const textResponseHandler = http.get('/api/schemas', () => {
        return new Response('Plain text response', { 
          status: 200,
          headers: { 'Content-Type': 'text/plain' }
        });
      });

      await withMockHandlers([textResponseHandler], async () => {
        const response = await apiClient.get('/schemas');
        expect(response.data).toBe('Plain text response');
      });
    });

    it('should handle missing required fields in response', async () => {
      const incompleteResponseHandler = http.get('/api/schemas', () => {
        return HttpResponse.json({
          // Missing 'success' and 'data' fields
          timestamp: Date.now()
        });
      });

      await withMockHandlers([incompleteResponseHandler], async () => {
        const response = await apiClient.get('/schemas');
        // Client should handle incomplete response gracefully
        expect(response.data).toBeDefined();
      });
    });

    it('should handle unexpected response structure', async () => {
      const unexpectedStructureHandler = http.get('/api/schemas', () => {
        return HttpResponse.json({
          // Completely different structure
          result: 'ok',
          payload: { schemas: [] },
          meta: { version: '1.0' }
        });
      });

      await withMockHandlers([unexpectedStructureHandler], async () => {
        const response = await apiClient.get('/schemas');
        expect(response.data).toEqual({
          result: 'ok',
          payload: { schemas: [] },
          meta: { version: '1.0' }
        });
      });
    });
  });

  describe('Authentication and Authorization Errors', () => {
    it('should handle missing authentication', async () => {
      const unauthenticatedHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { message: 'Authentication required' }
          },
          { status: 401 }
        );
      });

      await withMockHandlers([unauthenticatedHandler], async () => {
        const error = await apiClient.post('/mutation', {}).catch(e => e);
        expect(error).toBeInstanceOf(AuthenticationError);
      });
    });

    it('should handle expired authentication tokens', async () => {
      const expiredTokenHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Token expired',
              code: 'TOKEN_EXPIRED'
            }
          },
          { status: 401 }
        );
      });

      await withMockHandlers([expiredTokenHandler], async () => {
        const error = await apiClient.post('/mutation', {}).catch(e => e);
        expect(error).toBeInstanceOf(AuthenticationError);
        expect(error.message).toContain('Token expired');
      });
    });

    it('should handle insufficient permissions', async () => {
      const forbiddenHandler = http.post('/api/schema/user_profiles/approve', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { message: 'Insufficient permissions for schema approval' }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([forbiddenHandler], async () => {
        const error = await apiClient.post('/schema/user_profiles/approve').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(403);
      });
    });

    it('should handle invalid API keys', async () => {
      const invalidKeyHandler = http.get('/api/security/system-public-key', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Invalid API key',
              code: 'INVALID_API_KEY'
            }
          },
          { status: 401 }
        );
      });

      await withMockHandlers([invalidKeyHandler], async () => {
        const error = await apiClient.get('/security/system-public-key').catch(e => e);
        expect(error).toBeInstanceOf(AuthenticationError);
      });
    });
  });

  describe('Server-side Validation Errors', () => {
    it('should handle field validation errors', async () => {
      const validationHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Validation failed',
              validationErrors: {
                'schema': ['Schema name is required'],
                'data.email': ['Invalid email format'],
                'data.age': ['Age must be a positive number']
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([validationHandler], async () => {
        const error = await apiClient.post('/mutation', {}).catch(e => e);
        expect(error).toBeInstanceOf(ValidationError);
        expect(error.validationErrors).toEqual({
          'schema': ['Schema name is required'],
          'data.email': ['Invalid email format'],
          'data.age': ['Age must be a positive number']
        });
      });
    });

    it('should handle schema state validation errors', async () => {
      const schemaStateHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Schema not approved for mutations',
              code: 'SCHEMA_STATE_ERROR',
              details: {
                schemaName: 'events',
                currentState: 'available',
                operation: 'mutation'
              }
            }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([schemaStateHandler], async () => {
        const error = await apiClient.post('/mutation', { schema: 'events' }).catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(403);
        expect(error.details).toEqual({
          schemaName: 'events',
          currentState: 'available',
          operation: 'mutation'
        });
      });
    });

    it('should handle data format validation errors', async () => {
      const formatHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Invalid data format',
              details: {
                expected: 'object',
                received: 'string',
                field: 'data'
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([formatHandler], async () => {
        const error = await apiClient.post('/mutation', { data: 'invalid' }).catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.details).toEqual({
          expected: 'object',
          received: 'string',
          field: 'data'
        });
      });
    });
  });

  describe('Rate Limiting and Quota Errors', () => {
    it('should handle rate limiting with exponential backoff', async () => {
      let requestCount = 0;
      const rateLimitHandler = http.get('/api/schemas', () => {
        requestCount++;
        if (requestCount <= 2) {
          return HttpResponse.json(
            {
              success: false,
              error: { message: 'Rate limit exceeded' }
            },
            { 
              status: 429,
              headers: { 'Retry-After': '1' }
            }
          );
        }
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([rateLimitHandler], async () => {
        // Should eventually succeed after retries
        const response = await apiClient.get('/schemas');
        expect(response.success).toBe(true);
        expect(requestCount).toBe(3); // Initial + 2 retries
      });
    });

    it('should handle quota exceeded errors', async () => {
      const quotaHandler = http.post('/api/mutation', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Monthly quota exceeded',
              code: 'QUOTA_EXCEEDED',
              details: {
                quotaType: 'mutations',
                limit: 1000,
                used: 1000,
                resetDate: '2025-02-01T00:00:00Z'
              }
            }
          },
          { status: 429 }
        );
      });

      await withMockHandlers([quotaHandler], async () => {
        const error = await apiClient.post('/mutation', {}).catch(e => e);
        expect(error).toBeInstanceOf(RateLimitError);
        expect(error.details.quotaType).toBe('mutations');
      });
    });

    it('should handle concurrent request limits', async () => {
      const concurrencyHandler = http.get('/api/schemas', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Too many concurrent requests',
              code: 'CONCURRENCY_LIMIT'
            }
          },
          { status: 429 }
        );
      });

      await withMockHandlers([concurrencyHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        expect(error).toBeInstanceOf(RateLimitError);
        expect(error.code).toBe('RATE_LIMIT_ERROR');
      });
    });
  });

  describe('Retry Mechanism Testing', () => {
    it('should retry on retryable errors', async () => {
      let attemptCount = 0;
      const retryHandler = http.get('/api/schemas', () => {
        attemptCount++;
        if (attemptCount <= 2) {
          return HttpResponse.json(
            { success: false, error: { message: 'Server error' } },
            { status: 500 }
          );
        }
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([retryHandler], async () => {
        const response = await apiClient.get('/schemas');
        expect(response.success).toBe(true);
        expect(attemptCount).toBe(3); // Initial + 2 retries
      });
    });

    it('should not retry on non-retryable errors', async () => {
      let attemptCount = 0;
      const noRetryHandler = http.get('/api/schemas', () => {
        attemptCount++;
        return HttpResponse.json(
          { success: false, error: { message: 'Bad request' } },
          { status: 400 }
        );
      });

      await withMockHandlers([noRetryHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(attemptCount).toBe(1); // No retries for 400 errors
      });
    });

    it('should implement exponential backoff', async () => {
      const timestamps = [];
      let attemptCount = 0;
      
      const backoffHandler = http.get('/api/schemas', () => {
        timestamps.push(Date.now());
        attemptCount++;
        if (attemptCount <= 2) {
          return HttpResponse.json(
            { success: false, error: { message: 'Server error' } },
            { status: 500 }
          );
        }
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([backoffHandler], async () => {
        const startTime = Date.now();
        await apiClient.get('/schemas');
        
        // Verify exponential backoff delays
        expect(timestamps.length).toBe(3);
        const delay1 = timestamps[1] - timestamps[0];
        const delay2 = timestamps[2] - timestamps[1];
        
        // Second delay should be approximately 2x the first delay
        expect(delay2).toBeGreaterThan(delay1);
      });
    });

    it('should respect maximum retry attempts', async () => {
      let attemptCount = 0;
      const maxRetriesHandler = http.get('/api/schemas', () => {
        attemptCount++;
        return HttpResponse.json(
          { success: false, error: { message: 'Server error' } },
          { status: 500 }
        );
      });

      await withMockHandlers([maxRetriesHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(attemptCount).toBe(3); // Initial + 2 retries (maxRetries = 2)
      });
    });
  });

  describe('Concurrent Request Handling', () => {
    it('should handle multiple simultaneous requests', async () => {
      let requestCount = 0;
      const concurrentHandler = http.get('/api/schemas', async () => {
        requestCount++;
        await delay(100); // Simulate processing time
        return HttpResponse.json({ 
          success: true, 
          data: [],
          requestId: requestCount
        });
      });

      await withMockHandlers([concurrentHandler], async () => {
        // Make 5 concurrent requests
        const requests = Array(5).fill().map(() => apiClient.get('/schemas'));
        const responses = await Promise.all(requests);
        
        expect(responses).toHaveLength(5);
        responses.forEach(response => {
          expect(response.success).toBe(true);
        });
        expect(requestCount).toBe(5);
      });
    });

    it('should deduplicate identical concurrent requests', async () => {
      let requestCount = 0;
      const dedupeHandler = http.get('/api/schemas', async () => {
        requestCount++;
        await delay(100);
        return HttpResponse.json({ 
          success: true, 
          data: [],
          timestamp: Date.now()
        });
      });

      await withMockHandlers([dedupeHandler], async () => {
        // Make identical concurrent requests - should be deduplicated
        const requests = Array(3).fill().map(() => apiClient.get('/schemas'));
        const responses = await Promise.all(requests);
        
        expect(responses).toHaveLength(3);
        // All responses should be identical (deduplicated)
        const firstTimestamp = responses[0].data.timestamp;
        responses.forEach(response => {
          expect(response.data.timestamp).toBe(firstTimestamp);
        });
        expect(requestCount).toBe(1); // Only one actual request made
      });
    });

    it('should handle request cancellation', async () => {
      const controller = new AbortController();
      
      const cancellationHandler = http.get('/api/schemas', async () => {
        await delay(2000); // Long delay
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([cancellationHandler], async () => {
        // Start request and cancel it
        const requestPromise = apiClient.get('/schemas', {
          abortSignal: controller.signal
        });
        
        setTimeout(() => controller.abort(), 100);
        
        await expect(requestPromise).rejects.toThrow(TimeoutError);
      });
    });
  });

  describe('Error Recovery and Circuit Breaker Patterns', () => {
    it('should recover after temporary service outage', async () => {
      let isServiceDown = true;
      const recoveryHandler = http.get('/api/schemas', () => {
        if (isServiceDown) {
          return HttpResponse.json(
            { success: false, error: { message: 'Service unavailable' } },
            { status: 503 }
          );
        }
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([recoveryHandler], async () => {
        // First request should fail
        const error1 = await apiClient.get('/schemas').catch(e => e);
        expect(error1.status).toBe(503);
        
        // Restore service
        isServiceDown = false;
        
        // Second request should succeed
        const response2 = await apiClient.get('/schemas');
        expect(response2.success).toBe(true);
      });
    });

    it('should maintain request metrics for monitoring', async () => {
      const metricsHandler = http.get('/api/schemas', () => {
        return HttpResponse.json({ success: true, data: [] });
      });

      await withMockHandlers([metricsHandler], async () => {
        await apiClient.get('/schemas');
        
        const metrics = apiClient.getMetrics();
        expect(metrics).toHaveLength(1);
        expect(metrics[0]).toMatchObject({
          method: 'GET',
          url: expect.stringContaining('/schemas'),
          status: 200,
          duration: expect.any(Number)
        });
      });
    });

    it('should handle error escalation patterns', async () => {
      let errorCount = 0;
      const escalationHandler = http.get('/api/schemas', () => {
        errorCount++;
        
        // Escalate error severity over time
        if (errorCount === 1) {
          return HttpResponse.json(
            { success: false, error: { message: 'Temporary glitch' } },
            { status: 503 }
          );
        } else if (errorCount === 2) {
          return HttpResponse.json(
            { success: false, error: { message: 'System overload' } },
            { status: 503 }
          );
        } else {
          return HttpResponse.json(
            { success: false, error: { message: 'Critical system failure' } },
            { status: 500 }
          );
        }
      });

      await withMockHandlers([escalationHandler], async () => {
        // Each retry should encounter escalating errors
        const error = await apiClient.get('/schemas').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(errorCount).toBe(3); // All retry attempts exhausted
      });
    });
  });

  describe('Error Integration with UI Components', () => {
    it('should provide user-friendly error messages', async () => {
      const userFriendlyHandler = http.get('/api/schemas', () => {
        return HttpResponse.json(
          { success: false, error: { message: 'Database connection failed' } },
          { status: 500 }
        );
      });

      await withMockHandlers([userFriendlyHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        const userMessage = error.toUserMessage();
        expect(userMessage).toBe(ERROR_MESSAGES.SERVER_ERROR);
      });
    });

    it('should provide structured error data for logging', async () => {
      const structuredHandler = http.get('/api/schemas', () => {
        return HttpResponse.json(
          { 
            success: false, 
            error: { 
              message: 'Validation failed',
              code: 'VALIDATION_ERROR',
              details: { field: 'name', value: '' }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([structuredHandler], async () => {
        const error = await apiClient.get('/schemas').catch(e => e);
        const errorJson = error.toJSON();
        
        expect(errorJson).toMatchObject({
          name: 'ApiError',
          status: 400,
          code: 'VALIDATION_ERROR',
          details: { field: 'name', value: '' },
          timestamp: expect.any(Number),
          isRetryable: false
        });
      });
    });
  });
});