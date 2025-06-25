/**
 * TextField Component
 * Reusable text input field with validation and debouncing
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState, useEffect, useCallback } from 'react';
import FieldWrapper from './FieldWrapper.jsx';
import { COMPONENT_STYLES, FORM_FIELD_DEBOUNCE_MS } from '../../constants/ui.js';

/**
 * @typedef {Object} TextFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {string} value - Current field value
 * @property {function} onChange - Callback when value changes (value) => void
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 * @property {string} [error] - Error message to display
 * @property {string} [placeholder] - Placeholder text
 * @property {string} [helpText] - Help text to display
 * @property {'text'|'number'|'email'|'password'} [type] - Input type
 * @property {boolean} [debounced] - Whether to debounce onChange calls
 * @property {number} [debounceMs] - Debounce delay in milliseconds
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable text input field component with debouncing support
 * 
 * @param {TextFieldProps} props
 * @returns {JSX.Element}
 */
function TextField({
  name,
  label,
  value,
  onChange,
  required = false,
  disabled = false,
  error,
  placeholder,
  helpText,
  type = 'text',
  debounced = false,
  debounceMs = FORM_FIELD_DEBOUNCE_MS,
  className = ''
}) {
  const [internalValue, setInternalValue] = useState(value);
  const [isDebouncing, setIsDebouncing] = useState(false);

  // Update internal value when external value changes
  useEffect(() => {
    setInternalValue(value);
  }, [value]);

  // Debounced onChange handler
  const debouncedOnChange = useCallback(() => {
    let timeoutId;
    return (newValue) => {
      setIsDebouncing(true);
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        onChange(newValue);
        setIsDebouncing(false);
      }, debounceMs);
    };
  }, [onChange, debounceMs]);

  const debouncedCallback = debouncedOnChange();

  const handleChange = (e) => {
    const newValue = e.target.value;
    setInternalValue(newValue);
    
    if (debounced) {
      debouncedCallback(newValue);
    } else {
      onChange(newValue);
    }
  };

  const fieldId = `field-${name}`;
  const hasError = Boolean(error);

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
      error={error}
      helpText={helpText}
      className={className}
    >
      <div className="relative">
        <input
          id={fieldId}
          name={name}
          type={type}
          value={internalValue}
          onChange={handleChange}
          placeholder={placeholder}
          required={required}
          disabled={disabled}
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
        
        {/* Debouncing indicator */}
        {debounced && isDebouncing && (
          <div className="absolute right-2 top-1/2 transform -translate-y-1/2">
            <div className="animate-spin h-4 w-4 border-2 border-primary border-t-transparent rounded-full"></div>
          </div>
        )}
      </div>
    </FieldWrapper>
  );
}

export default TextField;