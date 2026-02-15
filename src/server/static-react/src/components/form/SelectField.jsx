/**
 * SelectField Component
 * Reusable select/dropdown field with loading states and accessibility
 * Part of TASK-002: Component Extraction and Modularization
 * TASK-009: Simplified using extracted utilities and hooks
 */

import FieldWrapper from './FieldWrapper.jsx';
import { UI_STATES } from '../../constants/ui.js';
import { useSearchableSelect } from '../../hooks/useSearchableSelect.js';
import { generateFieldId } from '../../utils/formHelpers.js';
import {
  processSelectConfig,
  getSelectStyles,
  createAriaAttributes,
  groupOptions
} from '../../utils/selectFieldHelpers.js';

/**
 * @typedef {Object} SelectOption
 * @property {string} value - Option value
 * @property {string} label - Option display text
 * @property {boolean} [disabled] - Whether option is disabled
 * @property {string} [group] - Optional group for grouping options
 */

/**
 * @typedef {Object} SelectFieldConfig
 * @property {boolean} [searchable] - Enable search functionality
 * @property {string} [placeholder] - Placeholder text
 * @property {string} [emptyMessage] - Message when no options available
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 * @property {boolean} [loading] - Whether options are loading
 */

/**
 * @typedef {Object} SelectFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {string} value - Current selected value
 * @property {SelectOption[]} options - Array of select options
 * @property {function} onChange - Callback when selection changes (value) => void
 * @property {string} [error] - Error message to display
 * @property {string} [helpText] - Help text to display
 * @property {SelectFieldConfig} [config] - Configuration options
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Simplified select field component with reduced complexity
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
  error,
  helpText,
  config = {},
  className = ''
}) {
  // Process configuration with defaults
  const processedConfig = processSelectConfig(config);
  const { searchable, placeholder, emptyMessage, required, disabled, loading } = processedConfig;

  // Generate consistent field ID and determine state
  const fieldId = generateFieldId(name);
  const hasError = Boolean(error);
  const hasOptions = options.length > 0;

  // Use searchable select hook if searchable is enabled
  const searchableSelect = useSearchableSelect(options, onChange, true);

  // Handle standard select change
  const handleStandardChange = (e) => {
    onChange(e.target.value);
  };

  // Render loading state
  if (loading) {
    return (
      <FieldWrapper label={label} name={name} required={required} error={error} helpText={helpText} className={className}>
        <div className="select opacity-50 cursor-not-allowed flex items-center">
          <span className="spinner mr-2" />
          {UI_STATES.loading}
        </div>
      </FieldWrapper>
    );
  }

  // Render empty state
  if (!hasOptions) {
    return (
      <FieldWrapper label={label} name={name} required={required} error={error} helpText={helpText} className={className}>
        <div className="select opacity-50 cursor-not-allowed">{emptyMessage}</div>
      </FieldWrapper>
    );
  }

  // Render searchable select
  if (searchable) {
    const { state, handleSearchChange, handleOptionSelect } = searchableSelect;
    
    return (
      <FieldWrapper label={label} name={name} required={required} error={error} helpText={helpText} className={className}>
        <div className="relative">
          <input
            type="text"
            placeholder={`Search ${label.toLowerCase()}...`}
            value={state.searchTerm}
            onChange={handleSearchChange}
            onFocus={() => searchableSelect.actions.openDropdown()}
            className={`input ${hasError ? 'border-gruvbox-red' : ''}`}
          />
          {state.isOpen && state.filteredOptions.length > 0 && (
            <div className="absolute z-10 w-full mt-1 bg-surface border border-border shadow-lg max-h-60 overflow-auto">
              {Object.entries(state.groupedOptions).map(([groupName, groupOptions]) => (
                <div key={groupName}>
                  {groupName !== 'default' && (
                    <div className="px-3 py-2 text-xs font-semibold text-tertiary bg-surface-secondary border-b">
                      {groupName}
                    </div>
                  )}
                  {groupOptions.map((option) => (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => handleOptionSelect(option)}
                      disabled={option.disabled}
                      className={`w-full text-left px-3 py-2 hover:bg-surface-secondary focus:bg-surface-secondary focus:outline-none ${
                        option.disabled ? 'text-tertiary cursor-not-allowed' : 'text-primary'
                      } ${value === option.value ? 'bg-gruvbox-orange text-surface' : ''}`}
                    >
                      {option.label}
                    </button>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
      </FieldWrapper>
    );
  }

  // Render standard select
  const groupedOptions = groupOptions(options);
  const selectStyles = getSelectStyles(null, hasError, disabled, loading);
  const ariaAttributes = createAriaAttributes(fieldId, hasError, helpText);

  return (
    <FieldWrapper label={label} name={name} required={required} error={error} helpText={helpText} className={className}>
      <select
        id={fieldId}
        name={name}
        value={value}
        onChange={handleStandardChange}
        required={required}
        disabled={disabled}
        className={selectStyles}
        {...ariaAttributes}
      >
        <option value="" disabled={required}>
          {placeholder}
        </option>
        {Object.entries(groupedOptions).map(([groupName, groupOptions]) =>
          groupName !== 'default' ? (
            <optgroup key={groupName} label={groupName}>
              {groupOptions.map((option) => (
                <option key={option.value} value={option.value} disabled={option.disabled}>
                  {option.label}
                </option>
              ))}
            </optgroup>
          ) : (
            groupOptions.map((option) => (
              <option key={option.value} value={option.value} disabled={option.disabled}>
                {option.label}
              </option>
            ))
          )
        )}
      </select>
    </FieldWrapper>
  );
}

export default SelectField;