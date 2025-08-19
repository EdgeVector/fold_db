/**
 * @fileoverview API Client-Specific Error Testing
 * 
 * Tests error handling for individual API clients (SchemaClient, SecurityClient, etc.)
 * Focuses on client-specific error scenarios and error propagation patterns.
 * 
 * @module clientErrorTesting
 * @since 2.0.0
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { http, HttpResponse, delay } from 'msw';
import { 
  schemaClient,
  securityClient,
  systemClient,
  transformClient,
  ingestionClient
} from '../../api/clients';
import { 
  ApiError, 
  NetworkError, 
  TimeoutError, 
  AuthenticationError,
  SchemaStateError,
  ValidationError,
  RateLimitError
} from '../../api/core/errors';
import { HTTP_STATUS_CODES, SCHEMA_STATES } from '../../constants/api';
import { withMockHandlers } from '../mocks/apiMocks';

describe('API Client-Specific Error Testing', () => {
  
  describe('SchemaClient Error Handling', () => {
    it('should handle schema not found errors', async () => {
      const notFoundHandler = http.get('/api/schema/:schemaName', ({ params }) => {
        return HttpResponse.json(
          {
            success: false,
            error: { message: `Schema ${params.schemaName} not found` }
          },
          { status: 404 }
        );
      });

      await withMockHandlers([notFoundHandler], async () => {
        const error = await schemaClient.getSchema('nonexistent').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(404);
        expect(error.message).toContain('Schema nonexistent not found');
      });
    });

    it('should handle schema state validation errors during operations', async () => {
      const stateErrorHandler = http.post('/api/schema/:schemaName/approve', ({ params }) => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: `Schema ${params.schemaName} cannot be approved from state blocked`,
              code: 'SCHEMA_STATE_ERROR',
              details: {
                schemaName: params.schemaName,
                currentState: 'blocked',
                operation: 'approve'
              }
            }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([stateErrorHandler], async () => {
        const error = await schemaClient.approveSchema('blocked_schema').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(403);
        expect(error.details).toEqual({
          schemaName: 'blocked_schema',
          currentState: 'blocked',
          operation: 'approve'
        });
      });
    });

    it('should handle concurrent schema operations conflicts', async () => {
      const conflictHandler = http.post('/api/schema/:schemaName/approve', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Schema is currently being modified by another operation',
              code: 'CONCURRENT_MODIFICATION'
            }
          },
          { status: 409 }
        );
      });

      await withMockHandlers([conflictHandler], async () => {
        const error = await schemaClient.approveSchema('user_profiles').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(409);
        expect(error.code).toBe('CONCURRENT_MODIFICATION');
      });
    });

    it('should handle invalid schema state parameter', async () => {
      await expect(schemaClient.getSchemasByState('invalid_state')).rejects.toThrow(
        'Invalid schema state: invalid_state'
      );
    });

    it('should handle schema operation authorization failures', async () => {
      const authFailureHandler = http.post('/api/schema/:schemaName/block', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Insufficient permissions to block schema',
              code: 'INSUFFICIENT_PERMISSIONS'
            }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([authFailureHandler], async () => {
        const error = await schemaClient.blockSchema('user_profiles').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(403);
      });
    });
  });

  describe('SecurityClient Error Handling', () => {
    it('should handle invalid public key format errors', async () => {
      const invalidKeyHandler = http.post('/api/security/register-key', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Invalid public key format',
              validationErrors: {
                'publicKey': ['Must be a valid Ed25519 public key in base64 format']
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([invalidKeyHandler], async () => {
        const error = await securityClient.registerPublicKey({
          publicKey: 'invalid_key',
          signature: 'valid_signature'
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ValidationError);
        expect(error.validationErrors.publicKey).toContain(
          'Must be a valid Ed25519 public key in base64 format'
        );
      });
    });

    it('should handle signature verification failures', async () => {
      const verificationFailureHandler = http.post('/api/security/verify', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Signature verification failed',
              code: 'INVALID_SIGNATURE',
              details: {
                reason: 'Signature does not match message and public key'
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([verificationFailureHandler], async () => {
        const error = await securityClient.verifyMessage({
          message: 'test message',
          signature: 'invalid_signature',
          publicKey: 'valid_key',
          timestamp: Date.now()
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ApiError);
        expect(error.code).toBe('INVALID_SIGNATURE');
      });
    });

    it('should handle expired timestamps in signed messages', async () => {
      const expiredTimestampHandler = http.post('/api/security/verify', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Message timestamp is too old',
              code: 'TIMESTAMP_EXPIRED',
              details: {
                maxAge: 300,
                messageAge: 600
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([expiredTimestampHandler], async () => {
        const error = await securityClient.verifyMessage({
          message: 'test message',
          signature: 'valid_signature',
          publicKey: 'valid_key',
          timestamp: Date.now() - 600000 // 10 minutes ago
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ApiError);
        expect(error.code).toBe('TIMESTAMP_EXPIRED');
      });
    });

    it('should handle system key unavailability', async () => {
      const keyUnavailableHandler = http.get('/api/security/system-public-key', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'System public key is temporarily unavailable',
              code: 'KEY_UNAVAILABLE'
            }
          },
          { status: 503 }
        );
      });

      await withMockHandlers([keyUnavailableHandler], async () => {
        const error = await securityClient.getSystemPublicKey().catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(503);
      });
    });
  });

  describe('SystemClient Error Handling', () => {
    it('should handle database reset authorization failures', async () => {
      const resetDeniedHandler = http.post('/api/system/reset-database', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Database reset requires admin privileges',
              code: 'ADMIN_REQUIRED'
            }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([resetDeniedHandler], async () => {
        const error = await systemClient.resetDatabase({
          confirmationCode: 'RESET_CONFIRM'
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(403);
        expect(error.code).toBe('ADMIN_REQUIRED');
      });
    });

    it('should handle system logs access errors', async () => {
      const logsAccessHandler = http.get('/api/system/logs', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Log access restricted in production mode',
              code: 'LOGS_RESTRICTED'
            }
          },
          { status: 403 }
        );
      });

      await withMockHandlers([logsAccessHandler], async () => {
        const error = await systemClient.getLogs().catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.code).toBe('LOGS_RESTRICTED');
      });
    });

    it('should handle system status polling failures', async () => {
      let failureCount = 0;
      const statusPollingHandler = http.get('/api/system/status', () => {
        failureCount++;
        if (failureCount <= 2) {
          return HttpResponse.json(
            {
              success: false,
              error: { message: 'System status temporarily unavailable' }
            },
            { status: 503 }
          );
        }
        return HttpResponse.json({
          success: true,
          data: { status: 'healthy', uptime: 12345 }
        });
      });

      await withMockHandlers([statusPollingHandler], async () => {
        // Should eventually succeed after retries
        const response = await systemClient.getSystemStatus();
        expect(response.success).toBe(true);
        expect(failureCount).toBe(3);
      });
    });
  });

  describe('TransformClient Error Handling', () => {
    it('should handle transform validation errors', async () => {
      const transformValidationHandler = http.post('/api/transforms/queue', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Transform validation failed',
              validationErrors: {
                'sourceSchema': ['Source schema must be approved'],
                'transform.operations': ['At least one operation is required'],
                'outputSchema': ['Output schema name is reserved']
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([transformValidationHandler], async () => {
        const error = await transformClient.addToQueue({
          sourceSchema: 'available_schema',
          transform: { operations: [] },
          outputSchema: 'system_reserved'
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ValidationError);
        expect(error.validationErrors).toHaveProperty('sourceSchema');
        expect(error.validationErrors).toHaveProperty('transform.operations');
      });
    });

    it('should handle transform queue capacity errors', async () => {
      const queueFullHandler = http.post('/api/transforms/queue', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Transform queue is at capacity',
              code: 'QUEUE_FULL',
              details: {
                currentSize: 100,
                maxCapacity: 100
              }
            }
          },
          { status: 429 }
        );
      });

      await withMockHandlers([queueFullHandler], async () => {
        const error = await transformClient.addToQueue({
          sourceSchema: 'user_profiles',
          transform: { operations: [{ type: 'filter' }] }
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(RateLimitError);
        expect(error.details.currentSize).toBe(100);
      });
    });

    it('should handle transform execution failures', async () => {
      const executionFailureHandler = http.get('/api/transforms/:transformId', ({ params }) => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Transform execution failed',
              code: 'EXECUTION_FAILED',
              details: {
                transformId: params.transformId,
                error: 'Schema compatibility issue',
                stage: 'validation'
              }
            }
          },
          { status: 422 }
        );
      });

      await withMockHandlers([executionFailureHandler], async () => {
        const error = await transformClient.getTransform('failed_transform_123').catch(e => e);
        expect(error).toBeInstanceOf(ApiError);
        expect(error.code).toBe('EXECUTION_FAILED');
        expect(error.details.stage).toBe('validation');
      });
    });
  });

  describe('IngestionClient Error Handling', () => {
    it('should handle invalid configuration errors', async () => {
      const invalidConfigHandler = http.post('/api/ingestion/config', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Invalid ingestion configuration',
              validationErrors: {
                'openrouter.apiKey': ['API key is required'],
                'openrouter.model': ['Unsupported model specified'],
                'batchSize': ['Batch size must be between 1 and 100']
              }
            }
          },
          { status: 400 }
        );
      });

      await withMockHandlers([invalidConfigHandler], async () => {
        const error = await ingestionClient.saveConfig({
          openrouter: { apiKey: '', model: 'unsupported-model' },
          batchSize: 200
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ValidationError);
        expect(error.validationErrors).toHaveProperty('openrouter.apiKey');
        expect(error.validationErrors).toHaveProperty('batchSize');
      });
    });

    it('should handle data validation failures', async () => {
      const dataValidationHandler = http.post('/api/ingestion/validate', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Data validation failed',
              code: 'VALIDATION_FAILED',
              details: {
                invalidRows: [
                  { row: 1, errors: ['Missing required field: email'] },
                  { row: 3, errors: ['Invalid date format: created_at'] }
                ],
                totalRows: 100,
                validRows: 98
              }
            }
          },
          { status: 422 }
        );
      });

      await withMockHandlers([dataValidationHandler], async () => {
        const error = await ingestionClient.validateData({
          schema: 'user_profiles',
          data: [/* test data */]
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ApiError);
        expect(error.code).toBe('VALIDATION_FAILED');
        expect(error.details.invalidRows).toHaveLength(2);
      });
    });

    it('should handle ingestion processing errors', async () => {
      const processingErrorHandler = http.post('/api/ingestion/process', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'Ingestion processing failed',
              code: 'PROCESSING_FAILED',
              details: {
                stage: 'ai_validation',
                reason: 'OpenRouter API rate limit exceeded',
                retryAfter: 60
              }
            }
          },
          { status: 429 }
        );
      });

      await withMockHandlers([processingErrorHandler], async () => {
        const error = await ingestionClient.processIngestion({
          schema: 'user_profiles',
          data: [/* test data */],
          useAI: true
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(RateLimitError);
        expect(error.details.stage).toBe('ai_validation');
        expect(error.retryAfter).toBe(60);
      });
    });

    it('should handle external service dependency failures', async () => {
      const serviceDownHandler = http.post('/api/ingestion/process', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { 
              message: 'External service unavailable',
              code: 'SERVICE_UNAVAILABLE',
              details: {
                service: 'openrouter',
                status: 'down',
                estimatedRecovery: '2025-01-07T10:00:00Z'
              }
            }
          },
          { status: 503 }
        );
      });

      await withMockHandlers([serviceDownHandler], async () => {
        const error = await ingestionClient.processIngestion({
          schema: 'user_profiles',
          data: [],
          useAI: true
        }).catch(e => e);
        
        expect(error).toBeInstanceOf(ApiError);
        expect(error.status).toBe(503);
        expect(error.details.service).toBe('openrouter');
      });
    });
  });

  describe('Cross-Client Error Propagation', () => {
    it('should handle cascading errors across dependent operations', async () => {
      const schemaFailureHandler = http.get('/api/schema/:schemaName', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { message: 'Schema service unavailable' }
          },
          { status: 503 }
        );
      });

      await withMockHandlers([schemaFailureHandler], async () => {
        // Schema failure should affect mutation operations
        const schemaError = await schemaClient.getSchema('user_profiles').catch(e => e);
        expect(schemaError.status).toBe(503);
        
        // This demonstrates how schema errors would propagate to other operations
        // In a real scenario, mutation client would check schema state first
      });
    });

    it('should handle authentication errors affecting multiple clients', async () => {
      const authFailureHandler = http.all('*', () => {
        return HttpResponse.json(
          {
            success: false,
            error: { message: 'Authentication token expired' }
          },
          { status: 401 }
        );
      });

      await withMockHandlers([authFailureHandler], async () => {
        // All authenticated operations should fail
        const schemaError = await schemaClient.getSchema('user_profiles').catch(e => e);
        const securityError = await securityClient.registerPublicKey({
          publicKey: 'test', signature: 'test'
        }).catch(e => e);
        
        expect(schemaError).toBeInstanceOf(AuthenticationError);
        expect(securityError).toBeInstanceOf(AuthenticationError);
      });
    });
  });
});