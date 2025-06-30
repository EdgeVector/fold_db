/**
 * Hooks index file for easy importing
 * TASK-009: Added new simplified hooks for better separation of concerns
 */

export { useApprovedSchemas } from './useApprovedSchemas.js';
export { useRangeSchema } from './useRangeSchema.js';
export { useFormValidation } from './useFormValidation.js';
export { useQueryState } from './useQueryState.js';

// TASK-009: New simplified hooks
export { useFieldValidation } from './useFieldValidation.js';
export { useValidationDebounce } from './useValidationDebounce.js';
export { useSearchableSelect } from './useSearchableSelect.js';
export { useRangeMode } from './useRangeMode.js';

// Re-export default exports for convenience
export { default as useApprovedSchemasDefault } from './useApprovedSchemas.js';
export { default as useRangeSchemaDefault } from './useRangeSchema.js';
export { default as useFormValidationDefault } from './useFormValidation.js';
export { default as useQueryStateDefault } from './useQueryState.js';
export { default as useFieldValidationDefault } from './useFieldValidation.js';
export { default as useValidationDebounceDefault } from './useValidationDebounce.js';
export { default as useSearchableSelectDefault } from './useSearchableSelect.js';
export { default as useRangeModeDefault } from './useRangeMode.js';