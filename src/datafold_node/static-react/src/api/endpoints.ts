// Centralized API endpoint definitions
export const API_ENDPOINTS = {
  // Auth & Security
  VERIFY_MESSAGE: '/security/verify',
  GET_SYSTEM_PUBLIC_KEY: '/security/system-key',
  
  // Schemas
  SCHEMAS_BASE: '/schemas',
  SCHEMA_BY_NAME: (name: string) => `/schema/${name}`,
  SCHEMA_APPROVE: (name: string) => `/schema/${name}/approve`,
  SCHEMA_BLOCK: (name: string) => `/schema/${name}/block`,
  // Non-existent: load/unload/state/status removed
  
  // Operations
  QUERY: '/query',
  MUTATION: '/mutation',
  // Non-existent: execute removed
  
  // System
  SYSTEM_STATUS: '/system/status',
  SYSTEM_LOGS: '/logs',
  SYSTEM_LOGS_STREAM: '/logs/stream',
  SYSTEM_RESET_DATABASE: '/system/reset-database',
  SYSTEM_PRIVATE_KEY: '/system/private-key',
  SYSTEM_PUBLIC_KEY: '/system/public-key',
  
  // Transforms
  TRANSFORMS: '/transforms',
  TRANSFORMS_QUEUE: '/transforms/queue',
  TRANSFORMS_QUEUE_ADD: (id: string) => `/transforms/queue/${id}`,
  
  // Ingestion
  INGESTION_STATUS: '/ingestion/status',
  INGESTION_CONFIG: '/ingestion/config',
  INGESTION_VALIDATE: '/ingestion/validate',
  INGESTION_PROCESS: '/ingestion/process',
  
  // Logging
  LOGS_LEVEL: '/logs/level',
} as const;

// Type-safe endpoint access
export type ApiEndpoint = typeof API_ENDPOINTS[keyof typeof API_ENDPOINTS];
