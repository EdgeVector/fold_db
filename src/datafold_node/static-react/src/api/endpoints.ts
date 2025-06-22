// Centralized API endpoint definitions
export const API_ENDPOINTS = {
  // Auth & Security
  VERIFY_MESSAGE: '/api/security/verify-message',
  REGISTER_PUBLIC_KEY: '/api/security/system-key',
  GET_SYSTEM_PUBLIC_KEY: '/api/security/system-key',
  
  // Schemas
  SCHEMAS_BASE: '/api/schemas',
  SCHEMA_BY_NAME: (name: string) => `/api/schemas/${name}`,
  SCHEMA_APPROVE: (name: string) => `/api/schemas/${name}/approve`,
  SCHEMA_BLOCK: (name: string) => `/api/schemas/${name}/block`,
  SCHEMAS_BY_STATE: (state: string) => `/api/schemas/state/${state}`,
  SCHEMA_STATUS: '/api/schemas/status',
  
  // Operations  
  QUERY: '/api/query',
  MUTATION: '/api/mutation',
  EXECUTE: '/api/execute',
  
  // System
  SYSTEM_STATUS: '/api/system/status',
  SYSTEM_CONFIG: '/api/system/config',
} as const;

// Type-safe endpoint access
export type ApiEndpoint = typeof API_ENDPOINTS[keyof typeof API_ENDPOINTS];