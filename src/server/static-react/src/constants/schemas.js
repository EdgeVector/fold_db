/**
 * Schema-related constants
 * Section 2.1.12 - Use of Constants for Repeated or Special Values
 */

// Schema fetching and caching constants
export const SCHEMA_FETCH_RETRY_COUNT = 3;
export const SCHEMA_CACHE_DURATION_MS = 300000; // 5 minutes
export const FORM_VALIDATION_DEBOUNCE_MS = 500;
export const RANGE_SCHEMA_FIELD_PREFIX = 'range_';

// Schema state constants
export const SCHEMA_STATES = {
  AVAILABLE: 'available',
  APPROVED: 'approved',
  BLOCKED: 'blocked',
  LOADING: 'loading',
  ERROR: 'error'
};

// API endpoints - Use centralized endpoints for API-STD-1 compliance
import { API_ENDPOINTS } from '../api/endpoints';
export const SCHEMA_API_ENDPOINTS = {
  // No dedicated "available" route on the server; use list-all
  AVAILABLE: API_ENDPOINTS.LIST_SCHEMAS,
  PERSISTED: API_ENDPOINTS.LIST_SCHEMAS,
  SCHEMA_DETAIL: API_ENDPOINTS.LIST_SCHEMAS
};

// Range schema constants
export const RANGE_SCHEMA_CONFIG = {
  FIELD_TYPE: 'Range',
  MUTATION_WRAPPER_KEY: 'value'
};

// Form field types for validation
export const FIELD_TYPES = {
  STRING: 'string',
  NUMBER: 'number',
  BOOLEAN: 'boolean',
  RANGE: 'Range'
};