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
  registerPublicKey,
  getSystemPublicKey,
  validatePublicKeyFormat,
  validateKeyRegistrationRequest,
  validateSignedMessage,
  createKeyRegistrationRequest,
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

// Mutation Client (if exists)
export * from './mutationClient';

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