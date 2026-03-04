/**
 * @fileoverview Custom hook for range schema operations and utilities
 *
 * This hook provides comprehensive utilities for working with range schemas,
 * including detection, validation, formatting, and query/mutation handling.
 * Range schemas are a special type of schema that use a range_key for efficient
 * time-series and ordered data storage.
 *
 * **Range Schema Structure:**
 * - Contains a `range_key` field that acts as the primary ordering field
 * - All fields have `field_type: "Range"`
 * - Non-range_key fields are stored as JSON objects for backend processing
 * - Supports efficient range queries and targeted mutations
 *
 * TASK-002: Extracted from inline logic for reusability
 * TASK-006: Enhanced with comprehensive JSDoc documentation
 *
 * @module useRangeSchema
 * @since 2.0.0
 */

import { useCallback } from 'react';
// Hardcoded to break circular dependency
const FORM_VALIDATION_DEBOUNCE_MS = 500;
import {
  isRangeSchema,
  getRangeKey,
  getNonRangeKeyFields,
  getRangeFields,
  getNonRangeFields,
  validateRangeKey,
  formatRangeMutation,
  formatRangeQuery,
  getRangeSchemaInfo
} from '../utils/rangeSchemaHelpers.js';

/**
 * @typedef {Object} Schema
 * @property {string} name - Schema name
 * @property {Object} fields - Field definitions
 * @property {Object} [schema_type] - Schema type information
 * @property {string} [range_key] - Legacy range key field name
 */

/**
 * @typedef {Object} RangeSchemaInfo
 * @property {boolean} isRangeSchema - Whether this is a range schema
 * @property {string|null} rangeKey - Name of the range key field
 * @property {Array<[string, Object]>} rangeFields - Array of [fieldName, fieldDef] for range fields
 * @property {Object} nonRangeKeyFields - Object containing non-range-key fields
 * @property {number} totalFields - Total number of fields in schema
 */

/**
 * @typedef {Object} RangeProps
 * @property {Function} isRange - Check if schema is a range schema
 * @property {Function} getRangeKey - Get range key field name
 * @property {Function} getNonRangeKeyFields - Get non-range-key fields
 * @property {Function} validateRangeKey - Validate range key value
 * @property {Function} formatRangeMutation - Format mutation for range schema
 * @property {Function} formatRangeQuery - Format query for range schema
 * @property {Function} getRangeSchemaInfo - Get comprehensive range schema info
 * @property {Function} getRangeFields - Get all range fields
 * @property {Function} getNonRangeFields - Get all non-range fields
 * @property {number} debounceMs - Debounce delay for form validation
 */

/**
 * @typedef {Object} UseRangeSchemaResult
 * @property {Function} isRange - Check if schema is a range schema
 * @property {Function} min - Get minimum range value (placeholder for future use)
 * @property {Function} max - Get maximum range value (placeholder for future use)
 * @property {Function} step - Get range step value (placeholder for future use)
 * @property {RangeProps} rangeProps - Collection of range-related functions
 */

/**
 * Custom hook for range schema operations and utilities
 *
 * This hook provides a comprehensive suite of utilities for working with range schemas,
 * which are specialized schemas designed for time-series and ordered data. Range schemas
 * have unique characteristics that require special handling for mutations and queries.
 *
 * **Key Features:**
 * - Range schema detection and validation
 * - Range key extraction and validation
 * - Mutation formatting for backend compatibility
 * - Query formatting with range filters
 * - Field separation (range vs non-range)
 * - Form validation with debouncing
 *
 * **Range Schema Characteristics:**
 * - Contains a designated range_key field for ordering
 * - All fields have field_type: "Range"
 * - Non-range_key fields are wrapped in objects for backend processing
 * - Supports efficient range-based queries
 *
 * **Usage Examples:**
 * ```jsx
 * // Basic range detection
 * const { isRange } = useRangeSchema();
 * if (isRange(schema)) {
 *   // Handle as range schema
 * }
 *
 * // Range mutation formatting
 * const { rangeProps } = useRangeSchema();
 * const mutation = rangeProps.formatRangeMutation(
 *   schema,
 *   'Create',
 *   'user123',
 *   { score: 85 }
 * );
 *
 * // Range key validation
 * const error = rangeProps.validateRangeKey(rangeKeyValue, true);
 * if (error) {
 *   // Handle validation error
 * }
 * ```
 *
 * @function useRangeSchema
 * @returns {UseRangeSchemaResult} Hook result object with range utilities
 *
 * @example
 * ```jsx
 * function RangeMutationForm({ schema }) {
 *   const { isRange, rangeProps } = useRangeSchema();
 *   const [rangeKey, setRangeKey] = useState('');
 *   const [formData, setFormData] = useState({});
 *
 *   if (!isRange(schema)) {
 *     return <div>Not a range schema</div>;
 *   }
 *
 *   const handleSubmit = () => {
 *     const error = rangeProps.validateRangeKey(rangeKey, true);
 *     if (error) {
 *       alert(error);
 *       return;
 *     }
 *
 *     const mutation = rangeProps.formatRangeMutation(
 *       schema, 'Create', rangeKey, formData
 *     );
 *     // Submit mutation
 *   };
 *
 *   return (
 *     <form onSubmit={handleSubmit}>
 *       <input
 *         value={rangeKey}
 *         onChange={(e) => setRangeKey(e.target.value)}
 *         placeholder={`Enter ${rangeProps.getRangeKey(schema)}`}
 *       />
 *       // ... other form fields
 *     </form>
 *   );
 * }
 * ```
 *
 * @since 2.0.0
 */
export function useRangeSchema() {
  // All range schema functionality now uses consolidated utilities
  // This eliminates the massive duplication between this hook and rangeSchemaUtils.js

  // Use the consolidated utilities directly
  const isRange = useCallback(isRangeSchema, []);
  
  // Placeholder functions for future range constraints
  const min = useCallback(() => null, []);
  const max = useCallback(() => null, []);
  const step = useCallback(() => null, []);

  // Collection of all range-related properties and functions
  // Now using consolidated utilities instead of duplicated implementations
  const rangeProps = {
    isRange: isRangeSchema,
    getRangeKey,
    getNonRangeKeyFields,
    validateRangeKey,
    formatRangeMutation,
    formatRangeQuery,
    getRangeSchemaInfo,
    getRangeFields,
    getNonRangeFields,
    debounceMs: FORM_VALIDATION_DEBOUNCE_MS
  };

  return {
    isRange,
    min,
    max,
    step,
    rangeProps
  };
}

export default useRangeSchema;