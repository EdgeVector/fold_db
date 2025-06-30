/**
 * useFieldValidation Hook
 * TASK-009: Additional Simplification - Extracted from useFormValidation complexity
 * 
 * Custom hook for single field validation with configurable rules.
 * This extraction reduces useFormValidation complexity and improves reusability.
 */

import { useState, useCallback } from 'react';
import {
  VALIDATION_MESSAGES,
  FIELD_TYPES,
  SCHEMA_STATES
} from '../constants/schemas.js';
import { normalizeSchemaState, isValueEmpty } from '../utils/rangeSchemaHelpers.js';

/**
 * @typedef {Object} ValidationRule
 * @property {('required'|'type'|'custom'|'schema_approved')} type - Type of validation to perform
 * @property {*} value - Value for the validation rule
 * @property {string} [message] - Custom error message to display
 * @property {Function} [validator] - Custom validator function for 'custom' type
 * @property {Array<Object>} [schemas] - Array of schemas for 'schema_approved' type
 */

/**
 * @typedef {Object} UseFieldValidationResult
 * @property {Function} validateField - Validate a field with given rules
 * @property {Function} validateRule - Validate a single rule
 * @property {Function} createRule - Factory function for creating validation rules
 * @property {string|null} lastError - Last validation error
 * @property {Function} clearError - Clear the last error
 */

/**
 * Custom hook for single field validation
 * 
 * Provides validation functionality for individual form fields with support
 * for various validation types including required, type checking, custom
 * validators, and schema approval validation.
 * 
 * @returns {UseFieldValidationResult} Hook result with validation functions
 * 
 * @example
 * ```jsx
 * function FormField({ value, onChange }) {
 *   const { validateField, createRule, lastError } = useFieldValidation();
 * 
 *   const rules = [
 *     createRule.required('This field is required'),
 *     createRule.type('string', 'Must be text')
 *   ];
 * 
 *   const handleChange = (newValue) => {
 *     onChange(newValue);
 *     validateField(newValue, rules);
 *   };
 * 
 *   return (
 *     <div>
 *       <input value={value} onChange={(e) => handleChange(e.target.value)} />
 *       {lastError && <span className="error">{lastError}</span>}
 *     </div>
 *   );
 * }
 * ```
 */
export function useFieldValidation() {
  const [lastError, setLastError] = useState(null);

  /**
   * Validates value type
   * @param {*} value - Value to validate
   * @param {string} expectedType - Expected type
   * @returns {string|null} Error message or null if valid
   */
  const validateType = useCallback((value, expectedType) => {
    if (isValueEmpty(value)) return null; // Skip type validation for empty values
    
    switch (expectedType) {
      case FIELD_TYPES.STRING:
        if (typeof value !== 'string') {
          return `Expected string, got ${typeof value}`;
        }
        break;
        
      case FIELD_TYPES.NUMBER:
        if (typeof value !== 'number' && !(!isNaN(Number(value)) && value !== '')) {
          return `Expected number, got ${typeof value}`;
        }
        break;
        
      case FIELD_TYPES.BOOLEAN:
        if (typeof value !== 'boolean' && value !== 'true' && value !== 'false') {
          return `Expected boolean, got ${typeof value}`;
        }
        break;
        
      default:
        // For custom types, skip validation
        break;
    }
    
    return null;
  }, []);

  /**
   * Checks if a schema is approved
   * @param {string} schemaName - Name of the schema
   * @param {Array} schemas - Array of available schemas
   * @returns {boolean} True if schema is approved
   */
  const isSchemaApproved = useCallback((schemaName, schemas = []) => {
    const schema = schemas.find(s => s.name === schemaName);
    if (!schema) return false;
    
    return normalizeSchemaState(schema.state) === SCHEMA_STATES.APPROVED;
  }, []);

  /**
   * Validates a single rule against a value
   * @param {*} value - Value to validate
   * @param {ValidationRule} rule - Validation rule
   * @returns {string|null} Error message or null if valid
   */
  const validateRule = useCallback((value, rule) => {
    switch (rule.type) {
      case 'required':
        if (rule.value && isValueEmpty(value)) {
          return rule.message || VALIDATION_MESSAGES.FIELD_REQUIRED;
        }
        break;
        
      case 'type': {
        const typeError = validateType(value, rule.value);
        if (typeError) {
          return rule.message || typeError;
        }
        break;
      }
        
      case 'custom':
        if (rule.validator && typeof rule.validator === 'function') {
          const customError = rule.validator(value);
          if (customError) {
            return rule.message || customError;
          }
        }
        break;
        
      case 'schema_approved':
        if (rule.value && !isSchemaApproved(value, rule.schemas)) {
          return rule.message || VALIDATION_MESSAGES.SCHEMA_NOT_APPROVED;
        }
        break;
        
      default:
        console.warn(`Unknown validation rule type: ${rule.type}`);
    }
    
    return null;
  }, [isSchemaApproved, validateType]);

  /**
   * Validates a field value against validation rules
   * @param {*} value - Value to validate
   * @param {ValidationRule[]} rules - Array of validation rules
   * @returns {string|null} Error message or null if valid
   */
  const validateField = useCallback((value, rules = []) => {
    for (const rule of rules) {
      const error = validateRule(value, rule);
      if (error) {
        setLastError(error);
        return error;
      }
    }
    
    setLastError(null);
    return null;
  }, [validateRule]);

  /**
   * Clear the last validation error
   */
  const clearError = useCallback(() => {
    setLastError(null);
  }, []);

  /**
   * Factory functions for creating common validation rules
   */
  const createRule = {
    required: (message) => ({ type: 'required', value: true, message }),
    type: (fieldType, message) => ({ type: 'type', value: fieldType, message }),
    custom: (validator, message) => ({ type: 'custom', validator, message }),
    schemaApproved: (schemas, message) => ({ type: 'schema_approved', value: true, schemas, message })
  };

  return {
    validateField,
    validateRule,
    createRule,
    lastError,
    clearError
  };
}

export default useFieldValidation;