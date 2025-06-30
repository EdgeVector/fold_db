// Centralized API endpoint definitions
export const API_ENDPOINTS = {
  // Auth & Security
  VERIFY_MESSAGE: '/security/verify-message',
  REGISTER_PUBLIC_KEY: '/security/system-key',
  GET_SYSTEM_PUBLIC_KEY: '/security/system-key',
  
  // Schemas
  SCHEMAS_BASE: '/schemas',
  SCHEMA_BY_NAME: (name: string) => `/schema/${name}`,
  SCHEMA_APPROVE: (name: string) => `/schema/${name}/approve`,
  SCHEMA_BLOCK: (name: string) => `/schema/${name}/block`,
  SCHEMA_LOAD: (name: string) => `/schema/${name}/load`,
  SCHEMA_UNLOAD: (name: string) => `/schema/${name}/unload`,
  SCHEMAS_BY_STATE: (state: string) => `/schemas/state/${state}`,
  SCHEMA_STATUS: '/schemas/status',
  
  // Operations
  QUERY: '/query',
  MUTATION: '/mutation',
  EXECUTE: '/execute',
  
  // System
  SYSTEM_STATUS: '/system/status',
  SYSTEM_CONFIG: '/system/config',
  SYSTEM_LOGS: '/logs',
  SYSTEM_LOGS_STREAM: '/logs/stream',
  SYSTEM_RESET_DATABASE: '/system/reset-database',
  
  // Transforms
  TRANSFORMS: '/transforms',
  TRANSFORMS_QUEUE: '/transforms/queue',
  TRANSFORMS_QUEUE_ADD: (id: string) => `/transforms/queue/${id}`,
  
  // Ingestion
  INGESTION_STATUS: '/ingestion/status',
  INGESTION_CONFIG: '/ingestion/config',
  INGESTION_VALIDATE: '/ingestion/validate',
  INGESTION_PROCESS: '/ingestion/process',
  
  // Network
  NETWORK_STATUS: '/network/status',
  NETWORK_PEERS: '/network/peers',
  NETWORK_CONNECT: '/network/connect',
  NETWORK_DISCONNECT: '/network/disconnect',
  
  // Logging
  LOGS_LEVEL: '/logs/level',
} as const;

// Type-safe endpoint access
export type ApiEndpoint = typeof API_ENDPOINTS[keyof typeof API_ENDPOINTS];