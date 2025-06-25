/**
 * Custom hook for form validation with debouncing
 * Centralizes form validation patterns used across components
 */

import { useState, useCallback, useRef } from 'react';
import { 
  FORM_VALIDATION_DEBOUNCE_MS,
  VALIDATION_MESSAGES,
  FIELD_TYPES,
  SCHEMA_STATES
} from '../constants/schemas.js';

/**
 * Validation rule structure
 * @typedef {Object} ValidationRule
 * @property {string} type - Type of validation ('required', 'type', 'custom', 'schema_approved')
 * @property {*} value - Value for the validation rule
 * @property {string} message - Custom error message
 * @property {Function} validator - Custom validator function
 */

/**
 * Hook for form validation with debouncing and schema-aware validation
 * 
 * @returns {Object} Hook result object
 * @returns {Function} validate - Validate single field with rules
 * @returns {Function} validateForm - Validate entire form against schema
 * @returns {Function} isFormValid - Check if form has no errors
 * @returns {Function} getFieldError - Get error for specific field
 * @returns {Object} errors - Current validation errors
 * @returns {Function} clearErrors - Clear all validation errors
 * @returns {Function} setFieldError - Set error for specific field
 */
export function useFormValidation() {
  const [errors, setErrors] = useState({});
  const debounceTimers = useRef({});

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
      for (const rule of rules) {
        const error = validateRule(value, rule);
        if (error) {
          setErrors(prev => ({ ...prev, [fieldName]: error }));
          return error;
        }
      }
      
      // Clear error if validation passes
      setErrors(prev => {
        const newErrors = { ...prev };
        delete newErrors[fieldName];
        return newErrors;
      });
      
      return null;
    };

    if (debounce) {
      // Clear existing timer
      if (debounceTimers.current[fieldName]) {
        clearTimeout(debounceTimers.current[fieldName]);
      }
      
      // Set new timer
      debounceTimers.current[fieldName] = setTimeout(() => {
        runValidation();
        delete debounceTimers.current[fieldName];
      }, FORM_VALIDATION_DEBOUNCE_MS);
      
      return null; // Return null for debounced validation
    } else {
      return runValidation();
    }
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
        
      case 'type':
        const typeError = validateType(value, rule.value);
        if (typeError) {
          return rule.message || typeError;
        }
        break;
        
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
  }, []);

  /**
   * Checks if a value is considered empty
   * @param {*} value - Value to check
   * @returns {boolean} True if value is empty
   */
  const isValueEmpty = useCallback((value) => {
    if (value === null || value === undefined) return true;
    if (typeof value === 'string') return value.trim().length === 0;
    if (Array.isArray(value)) return value.length === 0;
    if (typeof value === 'object') return Object.keys(value).length === 0;
    return false;
  }, []);

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
    
    const normalizeState = (state) => {
      if (typeof state === 'string') return state.toLowerCase();
      if (typeof state === 'object' && state !== null) return String(state).toLowerCase();
      return String(state || '').toLowerCase();
    };
    
    return normalizeState(schema.state) === SCHEMA_STATES.APPROVED;
  }, []);

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
        rules.push({ type: 'required', value: true });
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
          rules.push({ type: 'type', value: expectedType });
        }
      }
      
      // Add custom validation rules from config
      if (validationConfig.customRules && validationConfig.customRules[fieldName]) {
        rules.push(...validationConfig.customRules[fieldName]);
      }
      
      // Run validation for this field
      const error = validateRule(value, rules[0]); // Validate first rule that fails
      if (error) {
        formErrors[fieldName] = error;
      }
    });
    
    // Update errors state
    setErrors(formErrors);
    
    return formErrors;
  }, [validateRule]);

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
    
    // Clear any pending debounce timers
    Object.values(debounceTimers.current).forEach(timer => {
      clearTimeout(timer);
    });
    debounceTimers.current = {};
  }, []);

  /**
   * Sets error for a specific field
   * @param {string} fieldName - Name of the field
   * @param {string} errorMessage - Error message
   */
  const setFieldError = useCallback((fieldName, errorMessage) => {
    setErrors(prev => ({ ...prev, [fieldName]: errorMessage }));
  }, []);

  /**
   * Creates common validation rules for different field types
   */
  const createValidationRules = useCallback({
    required: (message) => ({ type: 'required', value: true, message }),
    type: (fieldType, message) => ({ type: 'type', value: fieldType, message }),
    custom: (validator, message) => ({ type: 'custom', validator, message }),
    schemaApproved: (schemas, message) => ({ type: 'schema_approved', value: true, schemas, message })
  }, []);

  return {
    validate,
    validateForm,
    isFormValid,
    getFieldError,
    errors,
    clearErrors,
    setFieldError,
    createValidationRules
  };
}

export default useFormValidation;