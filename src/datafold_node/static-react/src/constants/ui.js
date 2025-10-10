/**
 * UI Constants for Datafold React Application
 * Extracted from hardcoded values per Section 2.1.12
 * Part of TASK-005: Constants Extraction and Configuration Centralization
 *
 * Note: This file contains UI constants that are actively used.
 * These should be gradually migrated to more specific constant files.
 */

// Tab Configuration and Navigation
export const TAB_TRANSITION_DURATION_MS = 200;
export const FORM_FIELD_DEBOUNCE_MS = 300;

// Tab Definitions
export const DEFAULT_TABS = [
  { id: 'schemas', label: 'Schemas', icon: '📊' },
  { id: 'query', label: 'Query', icon: '🔍' },
  { id: 'llm-query', label: 'AI Query', icon: '🤖' },
  { id: 'mutation', label: 'Mutation', icon: '✏️' },
  { id: 'ingestion', label: 'Ingestion', icon: '📥' },
  { id: 'transforms', label: 'Transforms', icon: '🔄' },
  { id: 'keys', label: 'Key Management', icon: '🔑' }
];

// Button Text Constants
export const BUTTON_TEXT = {
  approve: 'Approve',
  block: 'Block',
  unload: 'Unload',
  executeQuery: 'Execute Query',
  executeMutation: 'Execute Mutation',
  confirm: 'Confirm',
  cancel: 'Cancel'
};

// Form Label Constants
export const FORM_LABELS = {
  schema: 'Schema',
  schemaEmpty: 'No schemas available',
  schemaHelp: 'Select a schema to work with',
  rangeKeyFilter: 'Range Key Filter',
  rangeKeyRequired: 'Range key is required',
  rangeKeyOptional: 'Range key is optional',
  operationType: 'Operation Type',
  operationHelp: 'Select the type of operation to perform'
};

// UI State Constants
export const UI_STATES = {
  loading: 'Loading...',
  error: 'Error',
  success: 'Success',
  idle: 'Ready'
};

// Mutation Type Constants
export const MUTATION_TYPES = [
  { value: 'Create', label: 'Create' },
  { value: 'Update', label: 'Update' },
  { value: 'Delete', label: 'Delete' }
];

// Schema Badge Colors
export const SCHEMA_BADGE_COLORS = {
  approved: 'bg-green-100 text-green-800',
  available: 'bg-blue-100 text-blue-800',
  blocked: 'bg-red-100 text-red-800',
  pending: 'bg-yellow-100 text-yellow-800'
};

// Authentication Indicators
export const AUTH_INDICATORS = {
  authenticated: '🔐',
  unauthenticated: '🔓',
  loading: '⏳'
};

// Help Text Constants
export const HELP_TEXT = {
  rangeSchema: 'Range schemas support filtering by a range key',
  mutation: 'Select an operation to perform on the schema',
  query: 'Query approved schemas for data',
  schemaStates: {
    approved: 'Schema is approved for use in queries and mutations',
    available: 'Schema is available but requires approval before use',
    blocked: 'Schema is blocked and cannot be used',
    pending: 'Schema approval is pending review',
    unknown: 'Schema state is unknown or invalid'
  }
};

// Range Schema Configuration
export const RANGE_SCHEMA_CONFIG = {
  label: 'Range Key',
  badgeColor: 'bg-purple-100 text-purple-800',
  indicator: {
    text: 'Range',
    className: 'ml-1 text-xs bg-purple-100 text-purple-800 px-1 py-0.5 rounded'
  },
  tooltip: 'This schema supports range-based queries'
};