/**
 * @fileoverview Custom hook for comprehensive form validation with debouncing
 *
 * This hook provides a complete form validation solution with support for
 * debounced validation, schema-aware validation, custom validators, and
 * integration with the application's schema system.
 *
 * TASK-002: Extracted from inline validation logic for reusability
 * TASK-006: Enhanced with comprehensive JSDoc documentation
 * TASK-009: Simplified using extracted focused hooks and utilities
 *
 * @module useFormValidation
 * @since 2.0.0
 * @see {@link https://github.com/datafold/datafold/docs/project_logic.md#schema-002} SCHEMA-002 compliance
 */

import { useState, useCallback } from 'react';
// Hardcoded to break circular dependency
const FIELD_TYPES = {
  STRING: 'string',
  NUMBER: 'number',
  BOOLEAN: 'boolean',
  RANGE: 'Range'
};
import { useFieldValidation } from './useFieldValidation.js';
import { useValidationDebounce } from './useValidationDebounce.js';

/**
 * @typedef {Object} ValidationConfig
 * @property {string[]} [requiredFields] - Array of field names that are required
 * @property {Object<string, ValidationRule[]>} [customRules] - Custom validation rules per field
 */

/**
 * @typedef {Object} UseFormValidationResult
 * @property {Function} validate - Validate single field with specified rules
 * @property {Function} validateForm - Validate entire form against schema definition
 * @property {Function} isFormValid - Check if current form state has no validation errors
 * @property {Function} getFieldError - Get validation error message for specific field
 * @property {Object<string, string>} errors - Current validation errors mapped by field name
 * @property {Function} clearErrors - Clear all validation errors from state
 * @property {Function} setFieldError - Set validation error for specific field
 * @property {Object} createValidationRules - Factory functions for creating common validation rules
 */

/**
 * Simplified form validation hook using focused validation utilities
 *
 * This hook now leverages smaller, focused hooks for individual concerns:
 * - useFieldValidation: Single field validation logic
 * - useValidationDebounce: Debouncing functionality
 *
 * This reduces complexity while maintaining the same external API.
 *
 * @function useFormValidation
 * @returns {UseFormValidationResult} Hook result object with validation functions and state
 */
export function useFormValidation() {
  const [errors, setErrors] = useState({});
  
  // Use focused validation hooks
  const fieldValidation = useFieldValidation();
  const debouncing = useValidationDebounce();

  /**
   * Validates a field value against specific validation rules
   * @param {string} fieldName - Name of the field
   * @param {*} value - Value to validate
   * @param {ValidationRule[]} rules - Array of validation rules
   * @param {boolean} debounce - Whether to debounce the validation
   * @returns {string|null} Error message or null if valid
   */
  const validate = useCallback((fieldName, value, rules = [], debounce = false) => {
    const runValidation = () => {
      const error = fieldValidation.validateField(value, rules);
      
      if (error) {
        setErrors(prev => ({ ...prev, [fieldName]: error }));
        return error;
      } else {
        // Clear error if validation passes
        setErrors(prev => {
          const newErrors = { ...prev };
          delete newErrors[fieldName];
          return newErrors;
        });
        return null;
      }
    };

    if (debounce) {
      debouncing.debounceValidation(runValidation, fieldName);
      return null; // Return null for debounced validation
    } else {
      return debouncing.executeImmediate(runValidation);
    }
  }, [fieldValidation, debouncing]);

  /**
   * Validates entire form data against a schema
   * @param {Object} data - Form data to validate
   * @param {Object} schema - Schema definition
   * @param {Object} validationConfig - Additional validation configuration
   * @returns {Object} Object with field names as keys and error messages as values
   */
  const validateForm = useCallback((data, schema, validationConfig = {}) => {
    const formErrors = {};
    
    if (!schema || !schema.fields) {
      return formErrors;
    }
    
    // Validate each field in the schema
    Object.entries(schema.fields).forEach(([fieldName, fieldDef]) => {
      const value = data[fieldName];
      const rules = [];
      
      // Add required validation if field is required
      if (validationConfig.requiredFields && validationConfig.requiredFields.includes(fieldName)) {
        rules.push(fieldValidation.createRule.required());
      }
      
      // Add type validation based on field definition
      if (fieldDef.field_type && fieldDef.field_type !== FIELD_TYPES.RANGE) {
        const typeMapping = {
          'String': FIELD_TYPES.STRING,
          'Number': FIELD_TYPES.NUMBER,
          'Boolean': FIELD_TYPES.BOOLEAN
        };
        
        const expectedType = typeMapping[fieldDef.field_type];
        if (expectedType) {
          rules.push(fieldValidation.createRule.type(expectedType));
        }
      }
      
      // Add custom validation rules from config
      if (validationConfig.customRules && validationConfig.customRules[fieldName]) {
        rules.push(...validationConfig.customRules[fieldName]);
      }
      
      // Run validation for this field
      if (rules.length > 0) {
        const error = fieldValidation.validateField(value, rules);
        if (error) {
          formErrors[fieldName] = error;
        }
      }
    });
    
    // Update errors state
    setErrors(formErrors);
    
    return formErrors;
  }, [fieldValidation]);

  /**
   * Checks if form is valid (no errors)
   * @param {Object} errorsToCheck - Errors object to check (defaults to current errors)
   * @returns {boolean} True if form is valid
   */
  const isFormValid = useCallback((errorsToCheck = errors) => {
    return Object.keys(errorsToCheck).length === 0;
  }, [errors]);

  /**
   * Gets error message for a specific field
   * @param {string} fieldName - Name of the field
   * @returns {string|null} Error message or null if no error
   */
  const getFieldError = useCallback((fieldName) => {
    return errors[fieldName] || null;
  }, [errors]);

  /**
   * Clears all validation errors
   */
  const clearErrors = useCallback(() => {
    setErrors({});
    debouncing.cancelDebounce();
  }, [debouncing]);

  /**
   * Sets error for a specific field
   * @param {string} fieldName - Name of the field
   * @param {string} errorMessage - Error message
   */
  const setFieldError = useCallback((fieldName, errorMessage) => {
    setErrors(prev => ({ ...prev, [fieldName]: errorMessage }));
  }, []);

  return {
    validate,
    validateForm,
    isFormValid,
    getFieldError,
    errors,
    clearErrors,
    setFieldError,
    createValidationRules: fieldValidation.createRule
  };
}

export default useFormValidation;