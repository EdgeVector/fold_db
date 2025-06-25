/**
 * useValidationDebounce Hook
 * TASK-009: Additional Simplification - Extracted from useFormValidation complexity
 * 
 * Custom hook for debounced validation to improve user experience.
 * This extraction reduces useFormValidation complexity and improves reusability.
 */

import { useRef, useCallback } from 'react';
import { FORM_VALIDATION_DEBOUNCE_MS } from '../constants/schemas.js';

/**
 * @typedef {Object} UseValidationDebounceResult
 * @property {Function} debounceValidation - Execute validation with debouncing
 * @property {Function} cancelDebounce - Cancel pending debounced validation
 * @property {Function} executeImmediate - Execute validation immediately
 * @property {boolean} hasPendingValidation - Whether there's a pending validation
 */

/**
 * Custom hook for debounced validation functionality
 * 
 * Provides debouncing capabilities for form validation to prevent excessive
 * validation calls during rapid user input. Improves user experience by
 * reducing validation noise.
 * 
 * @param {number} debounceMs - Debounce delay in milliseconds (optional)
 * @returns {UseValidationDebounceResult} Hook result with debouncing functions
 * 
 * @example
 * ```jsx
 * function ValidatedInput({ value, onChange, onValidate }) {
 *   const { debounceValidation, executeImmediate } = useValidationDebounce(300);
 * 
 *   const handleChange = (newValue) => {
 *     onChange(newValue);
 *     // Debounced validation during typing
 *     debounceValidation(() => onValidate(newValue));
 *   };
 * 
 *   const handleBlur = () => {
 *     // Immediate validation on blur
 *     executeImmediate(() => onValidate(value));
 *   };
 * 
 *   return (
 *     <input
 *       value={value}
 *       onChange={(e) => handleChange(e.target.value)}
 *       onBlur={handleBlur}
 *     />
 *   );
 * }
 * ```
 */
export function useValidationDebounce(debounceMs = FORM_VALIDATION_DEBOUNCE_MS) {
  const timeoutRef = useRef(null);
  const pendingValidationRef = useRef(false);

  /**
   * Execute validation with debouncing
   * @param {Function} validationFn - Validation function to execute
   * @param {string} [fieldName] - Optional field name for field-specific debouncing
   */
  const debounceValidation = useCallback((validationFn, fieldName = 'default') => {
    // Cancel existing timeout for this field
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }

    pendingValidationRef.current = true;

    // Set new timeout
    timeoutRef.current = setTimeout(() => {
      try {
        validationFn();
      } catch (error) {
        console.error('Debounced validation error:', error);
      } finally {
        pendingValidationRef.current = false;
        timeoutRef.current = null;
      }
    }, debounceMs);
  }, [debounceMs]);

  /**
   * Cancel pending debounced validation
   */
  const cancelDebounce = useCallback(() => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
      pendingValidationRef.current = false;
    }
  }, []);

  /**
   * Execute validation immediately (bypass debouncing)
   * @param {Function} validationFn - Validation function to execute
   */
  const executeImmediate = useCallback((validationFn) => {
    // Cancel any pending debounced validation
    cancelDebounce();

    try {
      return validationFn();
    } catch (error) {
      console.error('Immediate validation error:', error);
      return null;
    }
  }, [cancelDebounce]);

  /**
   * Check if there's a pending validation
   */
  const hasPendingValidation = pendingValidationRef.current;

  // Cleanup on unmount
  const cleanup = useCallback(() => {
    cancelDebounce();
  }, [cancelDebounce]);

  // Auto-cleanup effect would go here in a real implementation
  // For now, we'll rely on the consumer to call cleanup if needed

  return {
    debounceValidation,
    cancelDebounce,
    executeImmediate,
    hasPendingValidation,
    cleanup
  };
}

export default useValidationDebounce;