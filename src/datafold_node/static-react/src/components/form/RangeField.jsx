/**
 * RangeField Component
 * Reusable range input field for key ranges and filters
 * Part of TASK-002: Component Extraction and Modularization
 */

import { useState } from 'react';
import FieldWrapper from './FieldWrapper.jsx';
import TextField from './TextField.jsx';
import { COMPONENT_STYLES, HELP_TEXT } from '../../constants/ui.js';

/**
 * @typedef {Object} RangeValue
 * @property {string} [start] - Start of range
 * @property {string} [end] - End of range
 * @property {string} [key] - Exact key match
 * @property {string} [keyPrefix] - Key prefix match
 */

/**
 * @typedef {Object} RangeFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {RangeValue} value - Current range value
 * @property {function} onChange - Callback when value changes (value) => void
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 * @property {string} [error] - Error message to display
 * @property {string} [helpText] - Help text to display
 * @property {string} [rangeKeyName] - Name of the range key for display
 * @property {'range'|'key'|'prefix'|'all'} [mode] - Range input mode
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Reusable range input field component for filtering and key selection
 * 
 * @param {RangeFieldProps} props
 * @returns {JSX.Element}
 */
function RangeField({
  name,
  label,
  value = {},
  onChange,
  required = false,
  disabled = false,
  error,
  helpText,
  rangeKeyName = 'key',
  mode = 'all',
  className = ''
}) {
  const [activeMode, setActiveMode] = useState(
    value.start || value.end ? 'range' :
    value.key ? 'key' :
    value.keyPrefix ? 'prefix' :
    'range'
  );

  const handleModeChange = (newMode) => {
    setActiveMode(newMode);
    // Clear all values when changing modes
    onChange({});
  };

  const handleRangeChange = (field, newValue) => {
    const updatedValue = { ...value };
    
    // Clear other mode values when setting this mode
    if (field === 'start' || field === 'end') {
      delete updatedValue.key;
      delete updatedValue.keyPrefix;
    } else if (field === 'key') {
      delete updatedValue.start;
      delete updatedValue.end;
      delete updatedValue.keyPrefix;
    } else if (field === 'keyPrefix') {
      delete updatedValue.start;
      delete updatedValue.end;
      delete updatedValue.key;
    }
    
    updatedValue[field] = newValue;
    onChange(updatedValue);
  };

  const fieldId = `field-${name}`;
  const hasError = Boolean(error);

  // Build help text with range key name
  const buildHelpText = () => {
    if (helpText) return helpText;
    
    const help = { ...HELP_TEXT.rangeKeyFilter };
    return (
      <div className="space-y-1">
        <p><strong>Key Range:</strong> {help.keyRange}</p>
        <p><strong>Exact Key:</strong> {help.exactKey.replace('key', rangeKeyName)}</p>
        <p><strong>Key Prefix:</strong> {help.keyPrefix.replace('keys', `${rangeKeyName} values`)}</p>
        <p className="text-yellow-700"><strong>Note:</strong> {help.emptyNote}</p>
      </div>
    );
  };

  return (
    <FieldWrapper
      label={label}
      name={name}
      required={required}
      error={error}
      helpText={mode === 'all' ? buildHelpText() : helpText}
      className={className}
    >
      <div className="bg-yellow-50 rounded-lg p-4 space-y-4">
        {/* Range Key Display */}
        <div className="mb-3">
          <span className="text-sm font-medium text-gray-800">
            Range Key: {rangeKeyName}
          </span>
        </div>

        {/* Mode Selection (only if mode is 'all') */}
        {mode === 'all' && (
          <div className="flex space-x-4 mb-4">
            <button
              type="button"
              onClick={() => handleModeChange('range')}
              className={`px-3 py-1 text-xs rounded-md ${
                activeMode === 'range'
                  ? 'bg-primary text-white'
                  : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
              }`}
            >
              Key Range
            </button>
            <button
              type="button"
              onClick={() => handleModeChange('key')}
              className={`px-3 py-1 text-xs rounded-md ${
                activeMode === 'key'
                  ? 'bg-primary text-white'
                  : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
              }`}
            >
              Exact Key
            </button>
            <button
              type="button"
              onClick={() => handleModeChange('prefix')}
              className={`px-3 py-1 text-xs rounded-md ${
                activeMode === 'prefix'
                  ? 'bg-primary text-white'
                  : 'bg-gray-200 text-gray-700 hover:bg-gray-300'
              }`}
            >
              Key Prefix
            </button>
          </div>
        )}

        {/* Input Fields Based on Mode */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          {(mode === 'all' ? activeMode === 'range' : mode === 'range') && (
            <>
              <TextField
                name={`${name}-start`}
                label="Start Key"
                value={value.start || ''}
                onChange={(newValue) => handleRangeChange('start', newValue)}
                placeholder="Start key"
                disabled={disabled}
                className="col-span-1"
              />
              <TextField
                name={`${name}-end`}
                label="End Key"
                value={value.end || ''}
                onChange={(newValue) => handleRangeChange('end', newValue)}
                placeholder="End key"
                disabled={disabled}
                className="col-span-1"
              />
            </>
          )}

          {(mode === 'all' ? activeMode === 'key' : mode === 'key') && (
            <TextField
              name={`${name}-key`}
              label="Exact Key"
              value={value.key || ''}
              onChange={(newValue) => handleRangeChange('key', newValue)}
              placeholder={`Exact ${rangeKeyName} to match`}
              disabled={disabled}
              className="col-span-1"
            />
          )}

          {(mode === 'all' ? activeMode === 'prefix' : mode === 'prefix') && (
            <TextField
              name={`${name}-prefix`}
              label="Key Prefix"
              value={value.keyPrefix || ''}
              onChange={(newValue) => handleRangeChange('keyPrefix', newValue)}
              placeholder={`${rangeKeyName} prefix (e.g., 'user:')`}
              disabled={disabled}
              className="col-span-1"
            />
          )}
        </div>
      </div>
    </FieldWrapper>
  );
}

export default RangeField;