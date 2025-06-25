/**
 * SelectField Component
 * Reusable select/dropdown field with loading states and accessibility
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState } from 'react';
import FieldWrapper from './FieldWrapper.jsx';
import { COMPONENT_STYLES, UI_STATES } from '../../constants/ui.js';

/**
 * @typedef {Object} SelectOption
 * @property {string} value - Option value
 * @property {string} label - Option display text
 * @property {boolean} [disabled] - Whether option is disabled
 * @property {string} [group] - Optional group for grouping options
 */

/**
 * @typedef {Object} SelectFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {string} value - Current selected value
 * @property {SelectOption[]} options - Array of select options
 * @property {function} onChange - Callback when selection changes (value) => void
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 * @property {boolean} [loading] - Whether options are loading
 * @property {string} [error] - Error message to display
 * @property {string} [placeholder] - Placeholder text for empty state
 * @property {string} [helpText] - Help text to display
 * @property {boolean} [searchable] - Whether to enable search functionality
 * @property {string} [emptyMessage] - Message when no options available
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable select field component with loading states and grouping support
 * 
 * @param {SelectFieldProps} props
 * @returns {JSX.Element}
 */
function SelectField({
  name,
  label,
  value,
  options = [],
  onChange,
  required = false,
  disabled = false,
  loading = false,
  error,
  placeholder = 'Select an option...',
  helpText,
  searchable = false,
  emptyMessage = 'No options available',
  className = ''
}) {
  const [searchTerm, setSearchTerm] = useState('');
  const [isOpen, setIsOpen] = useState(false);

  const fieldId = `field-${name}`;
  const hasError = Boolean(error);
  const hasOptions = options.length > 0;

  // Filter options based on search term
  const filteredOptions = searchable && searchTerm
    ? options.filter(option => 
        option.label.toLowerCase().includes(searchTerm.toLowerCase()) ||
        option.value.toLowerCase().includes(searchTerm.toLowerCase())
      )
    : options;

  // Group options if they have group property
  const groupedOptions = filteredOptions.reduce((groups, option) => {
    const group = option.group || 'default';
    if (!groups[group]) {
      groups[group] = [];
    }
    groups[group].push(option);
    return groups;
  }, {});

  const handleChange = (e) => {
    const newValue = e.target.value;
    onChange(newValue);
    if (searchable) {
      setIsOpen(false);
    }
  };

  const handleSearchChange = (e) => {
    setSearchTerm(e.target.value);
  };

  // Determine select styling based on state
  const selectStyles = `${COMPONENT_STYLES.select.base} ${
    hasError 
      ? 'border-red-300 focus:ring-red-500 focus:border-red-500' 
      : ''
  } ${disabled || loading ? COMPONENT_STYLES.select.disabled : ''}`;

  // Show loading state
  if (loading) {
    return (
      <FieldWrapper
        label={label}
        name={name}
        required={required}
        error={error}
        helpText={helpText}
        className={className}
      >
        <div className={`${COMPONENT_STYLES.select.disabled} flex items-center`}>
          <div className="animate-spin h-4 w-4 border-2 border-gray-400 border-t-transparent rounded-full mr-2"></div>
          {UI_STATES.loading}
        </div>
      </FieldWrapper>
    );
  }

  // Show empty state
  if (!hasOptions) {
    return (
      <FieldWrapper
        label={label}
        name={name}
        required={required}
        error={error}
        helpText={helpText}
        className={className}
      >
        <div className={COMPONENT_STYLES.select.disabled}>
          {emptyMessage}
        </div>
      </FieldWrapper>
    );
  }

  return (
    <FieldWrapper
      label={label}
      name={name}
      required={required}
      error={error}
      helpText={helpText}
      className={className}
    >
      {searchable ? (
        // Custom searchable select implementation
        <div className="relative">
          <input
            type="text"
            placeholder={`Search ${label.toLowerCase()}...`}
            value={searchTerm}
            onChange={handleSearchChange}
            onFocus={() => setIsOpen(true)}
            className={`${COMPONENT_STYLES.input.base} ${hasError ? COMPONENT_STYLES.input.error : ''}`}
          />
          {isOpen && filteredOptions.length > 0 && (
            <div className="absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-md shadow-lg max-h-60 overflow-auto">
              {Object.entries(groupedOptions).map(([groupName, groupOptions]) => (
                <div key={groupName}>
                  {groupName !== 'default' && (
                    <div className="px-3 py-2 text-xs font-semibold text-gray-500 bg-gray-50 border-b">
                      {groupName}
                    </div>
                  )}
                  {groupOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => {
                        onChange(option.value);
                        setIsOpen(false);
                        setSearchTerm('');
                      }}
                      disabled={option.disabled}
                      className={`w-full text-left px-3 py-2 hover:bg-gray-100 focus:bg-gray-100 focus:outline-none ${
                        option.disabled ? 'text-gray-400 cursor-not-allowed' : 'text-gray-900'
                      } ${value === option.value ? 'bg-primary text-white' : ''}`}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
      ) : (
        // Standard select element
        <select
          id={fieldId}
          name={name}
          value={value}
          onChange={handleChange}
          required={required}
          disabled={disabled}
          className={selectStyles}
          aria-invalid={hasError}
          aria-describedby={
            hasError 
              ? `${fieldId}-error` 
              : helpText 
                ? `${fieldId}-help` 
                : undefined
          }
        >
          <option value="" disabled={required}>
            {placeholder}
          </option>
          
          {Object.entries(groupedOptions).map(([groupName, groupOptions]) => 
            groupName !== 'default' ? (
              <optgroup key={groupName} label={groupName}>
                {groupOptions.map((option) => (
                  <option 
                    key={option.value} 
                    value={option.value}
                    disabled={option.disabled}
                  >
                    {option.label}
                  </option>
                ))}
              </optgroup>
            ) : (
              groupOptions.map((option) => (
                <option 
                  key={option.value} 
                  value={option.value}
                  disabled={option.disabled}
                >
                  {option.label}
                </option>
              ))
            )
          )}
        </select>
      )}
    </FieldWrapper>
  );
}

export default SelectField;