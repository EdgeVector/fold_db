/**
 * NumberField Component
 * Reusable numeric input field with validation and formatting
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState, useEffect } from 'react';
import FieldWrapper from './FieldWrapper.jsx';
import { COMPONENT_STYLES } from '../../constants/ui.js';

/**
 * @typedef {Object} NumberFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {number|string} value - Current field value
 * @property {function} onChange - Callback when value changes (value) => void
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 * @property {string} [error] - Error message to display
 * @property {string} [placeholder] - Placeholder text
 * @property {string} [helpText] - Help text to display
 * @property {number} [min] - Minimum allowed value
 * @property {number} [max] - Maximum allowed value
 * @property {number} [step] - Step increment for input
 * @property {boolean} [allowFloat] - Whether to allow decimal values
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable numeric input field component with validation
 * 
 * @param {NumberFieldProps} props
 * @returns {JSX.Element}
 */
function NumberField({
  name,
  label,
  value,
  onChange,
  required = false,
  disabled = false,
  error,
  placeholder,
  helpText,
  min,
  max,
  step = 1,
  allowFloat = true,
  className = ''
}) {
  const [internalValue, setInternalValue] = useState(value?.toString() || '');
  const [validationError, setValidationError] = useState('');

  // Update internal value when external value changes
  useEffect(() => {
    setInternalValue(value?.toString() || '');
  }, [value]);

  const validateNumber = (stringValue) => {
    if (!stringValue.trim()) {
      return required ? 'This field is required' : '';
    }

    const numValue = allowFloat ? parseFloat(stringValue) : parseInt(stringValue, 10);
    
    if (isNaN(numValue)) {
      return `Please enter a valid ${allowFloat ? 'number' : 'integer'}`;
    }

    if (min !== undefined && numValue < min) {
      return `Value must be at least ${min}`;
    }

    if (max !== undefined && numValue > max) {
      return `Value must be no more than ${max}`;
    }

    if (!allowFloat && stringValue.includes('.')) {
      return 'Decimal values are not allowed';
    }

    return '';
  };

  const handleChange = (e) => {
    const newValue = e.target.value;
    setInternalValue(newValue);

    // Validate the input
    const validationErr = validateNumber(newValue);
    setValidationError(validationErr);

    // Convert to number if valid, otherwise pass empty string
    if (!validationErr && newValue.trim()) {
      const numValue = allowFloat ? parseFloat(newValue) : parseInt(newValue, 10);
      if (!isNaN(numValue)) {
        onChange(numValue);
        return;
      }
    }
    
    // Pass empty value for invalid input
    onChange('');
  };

  const handleBlur = () => {
    // Re-validate on blur to ensure final validation
    const validationErr = validateNumber(internalValue);
    setValidationError(validationErr);
  };

  const fieldId = `field-${name}`;
  const finalError = error || validationError;
  const hasError = Boolean(finalError);

  // Determine input styling based on state
  const inputStyles = `${COMPONENT_STYLES.input.base} ${
    hasError 
      ? COMPONENT_STYLES.input.error 
      : COMPONENT_STYLES.input.success
  } ${disabled ? 'bg-gray-100 cursor-not-allowed' : ''}`;

  return (
    <FieldWrapper
      label={label}
      name={name}
      required={required}
      error={finalError}
      helpText={helpText}
      className={className}
    >
      <div className="relative">
        <input
          id={fieldId}
          name={name}
          type="number"
          value={internalValue}
          onChange={handleChange}
          onBlur={handleBlur}
          placeholder={placeholder}
          required={required}
          disabled={disabled}
          min={min}
          max={max}
          step={allowFloat ? 'any' : step}
          className={inputStyles}
          aria-invalid={hasError}
          aria-describedby={
            hasError 
              ? `${fieldId}-error` 
              : helpText 
                ? `${fieldId}-help` 
                : undefined
          }
        />
        
        {/* Min/Max indicators */}
        {(min !== undefined || max !== undefined) && !hasError && (
          <div className="absolute right-2 top-1/2 transform -translate-y-1/2 text-xs text-gray-400">
            {min !== undefined && max !== undefined 
              ? `${min}-${max}`
              : min !== undefined 
                ? `≥${min}`
                : `≤${max}`
            }
          </div>
        )}
      </div>
    </FieldWrapper>
  );
}

export default NumberField;