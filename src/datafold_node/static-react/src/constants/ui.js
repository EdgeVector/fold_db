/**
 * UI Constants for Datafold React Application
 * Extracted from hardcoded values per Section 2.1.12
 * Part of TASK-002: Component Extraction and Modularization
 *
 * NOTE: This file is part of the legacy constants system.
 * For new development, use the centralized constants from './index.js'
 * TASK-005: Constants Extraction and Configuration Centralization
 */

// Tab Configuration and Navigation
export const TAB_TRANSITION_DURATION_MS = 200;
export const FORM_FIELD_DEBOUNCE_MS = 300;

// Tab Definitions
export const DEFAULT_TABS = [
  { id: 'schemas', label: 'Schemas', requiresAuth: true, icon: '📊' },
  { id: 'query', label: 'Query', requiresAuth: true, icon: '🔍' },
  { id: 'mutation', label: 'Mutation', requiresAuth: true, icon: '✏️' },
  { id: 'ingestion', label: 'Ingestion', requiresAuth: true, icon: '📥' },
  { id: 'transforms', label: 'Transforms', requiresAuth: true, icon: '🔄' },
  { id: 'dependencies', label: 'Dependencies', requiresAuth: true, icon: '🔗' },
  { id: 'keys', label: 'Keys', requiresAuth: false, icon: '🗝️' }
];

// Schema Badge Colors
export const SCHEMA_BADGE_COLORS = {
  approved: 'bg-green-100 text-green-800 border border-green-200',
  available: 'bg-blue-100 text-blue-800 border border-blue-200',
  blocked: 'bg-red-100 text-red-800 border border-red-200',
  pending: 'bg-yellow-100 text-yellow-800 border border-yellow-200'
};

// Component Z-Index Stack
export const COMPONENT_Z_INDEX = {
  dropdown: 10,
  modal: 50,
  tooltip: 100,
  overlay: 1000
};

// Form Field Labels and Messages
export const FORM_LABELS = {
  schema: 'Select Schema',
  schemaEmpty: 'No approved schemas available',
  schemaHelp: 'Only approved schemas can be used (SCHEMA-002)',
  operationType: 'Operation Type',
  operationHelp: 'Choose the type of operation to perform',
  fields: 'Select Fields',
  rangeKey: 'Range Key',
  rangeKeyRequired: 'Required',
  rangeKeyOptional: 'Optional for targeting',
  rangeKeyFilter: 'Range Key Filter',
  rangeFieldFilters: 'Range Field Filters'
};

// Button Text
export const BUTTON_TEXT = {
  executeQuery: 'Execute Query',
  executeMutation: 'Execute Mutation',
  approve: 'Approve',
  block: 'Block',
  unload: 'Unload',
  load: 'Load',
  cancel: 'Cancel',
  confirm: 'Confirm'
};

// Mutation Types
export const MUTATION_TYPES = [
  { value: 'Create', label: 'Create - Add new data' },
  { value: 'Update', label: 'Update - Modify existing data' },
  { value: 'Delete', label: 'Delete - Remove existing data' }
];

// Field Types and Their Display Properties
export const FIELD_TYPE_CONFIG = {
  String: { color: 'bg-blue-100 text-blue-800', icon: '📝' },
  Number: { color: 'bg-green-100 text-green-800', icon: '🔢' },
  Boolean: { color: 'bg-purple-100 text-purple-800', icon: '✓' },
  Range: { color: 'bg-orange-100 text-orange-800', icon: '📊' },
  Object: { color: 'bg-gray-100 text-gray-800', icon: '📦' },
  Array: { color: 'bg-pink-100 text-pink-800', icon: '📋' }
};

// Permission Policy Colors
export const PERMISSION_COLORS = {
  read: 'bg-blue-100 text-blue-800',
  write: 'bg-orange-100 text-orange-800',
  noRequirement: 'bg-gray-100 text-gray-800',
  distance: 'bg-purple-100 text-purple-800'
};

// Loading and Error States
export const UI_STATES = {
  loading: 'Loading...',
  error: 'An error occurred',
  noData: 'No data available',
  success: 'Operation completed successfully',
  unauthorized: 'Authentication required',
  forbidden: 'Access denied'
};

// Range Schema Indicators
export const RANGE_SCHEMA_CONFIG = {
  backgroundColor: 'bg-purple-50',
  borderColor: 'border-purple-200',
  badgeColor: 'bg-purple-200 text-purple-800',
  label: 'Range Key'
};

// Component Styling Constants
export const COMPONENT_STYLES = {
  tab: {
    base: 'px-4 py-2 text-sm font-medium transition-all duration-200',
    active: 'text-primary border-b-2 border-primary',
    inactive: 'text-gray-500 hover:text-gray-700 hover:border-gray-300',
    disabled: 'text-gray-300 cursor-not-allowed'
  },
  button: {
    primary: 'bg-primary hover:bg-primary/90 text-white',
    secondary: 'bg-gray-100 hover:bg-gray-200 text-gray-700',
    danger: 'bg-red-600 hover:bg-red-700 text-white',
    disabled: 'bg-gray-300 cursor-not-allowed text-gray-500'
  },
  input: {
    base: 'block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary',
    error: 'border-red-300 focus:ring-red-500 focus:border-red-500',
    success: 'border-green-300 focus:ring-green-500 focus:border-green-500'
  },
  select: {
    base: 'block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-primary focus:border-primary rounded-md',
    disabled: 'bg-gray-100 text-gray-500 cursor-not-allowed'
  }
};

// Authentication Indicators
export const AUTH_INDICATORS = {
  locked: '🔒',
  unlocked: '✓',
  pending: '⏳'
};

// Help Text for Complex Components
export const HELP_TEXT = {
  rangeKeyFilter: {
    keyRange: 'Matches keys between start and end (inclusive start, exclusive end)',
    exactKey: 'Matches a specific key exactly',
    keyPrefix: 'Matches all keys starting with the prefix',
    emptyNote: 'Leave all fields empty to query all data from this range schema.'
  },
  schemaStates: {
    approved: 'Schema is approved and ready for use',
    available: 'Schema is available but not yet approved',
    blocked: 'Schema is blocked and cannot be used',
    pending: 'Schema approval is pending'
  }
};

// Responsive Breakpoints (for consistent usage)
export const BREAKPOINTS = {
  sm: '640px',
  md: '768px',
  lg: '1024px',
  xl: '1280px'
};