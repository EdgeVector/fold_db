/**
 * API Clients Index
 * Centralized exports for all API clients
 * Part of API-STD-1 standardization
 */

// Schema Client
export { 
  schemaClient,
  UnifiedSchemaClient,
  createSchemaClient,
  getSchemasByState,
  getAllSchemasWithState,
  getSchemaStatus,
  getSchema,
  approveSchema,
  blockSchema,
  loadSchema,
  unloadSchema,
  getApprovedSchemas,
  validateSchemaForOperation
} from './schemaClient';

// Security Client
export {
  securityClient,
  UnifiedSecurityClient,
  createSecurityClient,
  verifyMessage,
  getSystemPublicKey,
  validatePublicKeyFormat,
  validateSignedMessage,
  getSecurityStatus
} from './securityClient';

// System Client
export {
  systemClient,
  UnifiedSystemClient,
  createSystemClient,
  getLogs,
  resetDatabase,
  getSystemStatus,
  createLogStream,
  validateResetRequest
} from './systemClient';

// Transform Client
export {
  transformClient,
  UnifiedTransformClient,
  createTransformClient,
  getTransforms,
  getQueue,
  addToQueue,
  refreshQueue,
  getTransform,
  removeFromQueue,
  validateTransformId
} from './transformClient';

// Mutation Client (if exists)
export * from './mutationClient';

// Ingestion Client
export {
  ingestionClient,
  UnifiedIngestionClient,
  createIngestionClient,
  getStatus,
  getConfig,
  saveConfig,
  validateData,
  processIngestion,
  validateIngestionRequest,
  createIngestionRequest
} from './ingestionClient';

// LLM Query Client
export { llmQueryClient } from './llmQueryClient';

// Type exports for convenience
export type {
  SchemasByStateResponse,
  SchemasWithStateResponse,
  SchemaStatusResponse
} from './schemaClient';

export type {
  SystemKeyResponse,
  KeyValidationResult,
  SecurityStatus
} from './securityClient';

export type {
  LogsResponse,
  ResetDatabaseRequest,
  ResetDatabaseResponse,
  SystemStatusResponse
} from './systemClient';

export type {
  Transform,
  TransformsResponse,
  QueueInfo,
  AddToQueueRequest,
  AddToQueueResponse
} from './transformClient';

export type {
  IngestionStatus,
  OpenRouterConfig,
  OllamaConfig,
  IngestionConfig,
  ValidationRequest,
  ValidationResponse,
  ProcessIngestionRequest,
  ProcessIngestionResponse
} from './ingestionClient';
