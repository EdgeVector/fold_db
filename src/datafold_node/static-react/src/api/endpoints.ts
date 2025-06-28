// Centralized API endpoint definitions
export const API_ENDPOINTS = {
  // Auth & Security
  VERIFY_MESSAGE: '/security/verify-message',
  REGISTER_PUBLIC_KEY: '/security/system-key',
  GET_SYSTEM_PUBLIC_KEY: '/security/system-key',
  
  // Schemas
  SCHEMAS_BASE: '/schemas',
  SCHEMA_BY_NAME: (name: string) => `/schemas/${name}`,
  SCHEMA_APPROVE: (name: string) => `/schemas/${name}/approve`,
  SCHEMA_BLOCK: (name: string) => `/schemas/${name}/block`,
  SCHEMAS_BY_STATE: (state: string) => `/schemas/state/${state}`,
  SCHEMA_STATUS: '/schemas/status',
  
  // Operations
  QUERY: '/query',
  MUTATION: '/mutation',
  EXECUTE: '/execute',
  
  // System
  SYSTEM_STATUS: '/system/status',
  SYSTEM_CONFIG: '/system/config',
} as const;

// Type-safe endpoint access
export type ApiEndpoint = typeof API_ENDPOINTS[keyof typeof API_ENDPOINTS];