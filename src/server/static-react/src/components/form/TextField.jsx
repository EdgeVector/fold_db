/**
 * TextField Component
 * Reusable text input field with validation and debouncing
 * Part of TASK-002: Component Extraction and Modularization
 * TASK-008: Updated to use consolidated form utilities
 */

import { useState, useEffect, useCallback, useRef } from 'react';
import FieldWrapper from './FieldWrapper.jsx';
import { FORM_FIELD_DEBOUNCE_MS } from '../../constants/ui.js';
import {
  generateFieldId,
  hasFieldError,
  generateInputStyles,
  generateAriaAttributes,
  getLoadingSpinnerClasses
} from '../../utils/formHelpers.js';

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
  const timeoutRef = useRef(null);
  const rafRef = useRef(null);
  const onChangeRef = useRef(onChange);
  
  // Keep onChangeRef current with the latest onChange
  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  // Cleanup RAF and timeout on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      if (rafRef.current && typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
        window.cancelAnimationFrame(rafRef.current);
      }
    };
  }, []);
  
  const debouncedOnChange = useCallback((newValue) => {
    setIsDebouncing(true);
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
      timeoutRef.current = null;
    }
    if (rafRef.current && typeof window !== 'undefined' && typeof window.cancelAnimationFrame === 'function') {
      window.cancelAnimationFrame(rafRef.current);
      rafRef.current = null;
    }
    const schedule = () => {
      timeoutRef.current = setTimeout(() => {
        onChangeRef.current(newValue);
        setIsDebouncing(false);
      }, debounceMs);
    };
    if (typeof window !== 'undefined' && typeof window.requestAnimationFrame === 'function') {
      rafRef.current = window.requestAnimationFrame(schedule);
    } else {
      setTimeout(schedule, 0);
    }
  }, [debounceMs]);

  const handleChange = (e) => {
    const newValue = e.target.value;
    setInternalValue(newValue);
    
    if (debounced) {
      debouncedOnChange(newValue);
    } else {
      onChange(newValue);
    }
  };

  const fieldId = generateFieldId(name);
  const hasError = hasFieldError(error);
  const inputStyles = generateInputStyles({ hasError, disabled });
  const ariaAttributes = generateAriaAttributes({
    fieldId,
    hasError,
    hasHelp: Boolean(helpText)
  });

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
          {...ariaAttributes}
        />
        
        {/* Debouncing indicator */}
        {debounced && isDebouncing && (
          <div className="absolute right-2 top-1/2 transform -translate-y-1/2">
            <div
              className={getLoadingSpinnerClasses({ size: 'md', color: 'primary' })}
              role="status"
              aria-label="Processing input"
            ></div>
          </div>
        )}
      </div>
    </FieldWrapper>
  );
}

export default TextField;