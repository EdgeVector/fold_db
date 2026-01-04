/**
 * Hooks index file for easy importing
 * TASK-009: Added new simplified hooks for better separation of concerns
 */

export { useApprovedSchemas } from './useApprovedSchemas.js';
export { useRangeSchema } from './useRangeSchema.js';
export { useQueryState } from './useQueryState.js';
export { useSearchableSelect } from './useSearchableSelect.js';
export { useRangeMode } from './useRangeMode.js';

// Re-export default exports for convenience
export { default as useApprovedSchemasDefault } from './useApprovedSchemas.js';
export { default as useRangeSchemaDefault } from './useRangeSchema.js';
export { default as useQueryStateDefault } from './useQueryState.js';
export { default as useSearchableSelectDefault } from './useSearchableSelect.js';
export { default as useRangeModeDefault } from './useRangeMode.js';