/**
 * RangeField Component
 * Reusable range input field for key ranges and filters
 * Part of TASK-002: Component Extraction and Modularization
 * TASK-009: Simplified using extracted utilities and hooks
 */

import FieldWrapper from './FieldWrapper.jsx';
import TextField from './TextField.jsx';
import { useRangeMode } from '../../hooks/useRangeMode.js';
import {
  generateRangeHelpText,
  getRangeModeConfig,
  getModeButtonStyles,
  getModeLabels,
  getVisibleFields,
  validateRangeConfig,
  getRangeFieldContainerStyles,
  getRangeKeyDisplayStyles,
  getModeSelectorStyles,
  getInputGridStyles
} from '../../utils/rangeFieldHelpers.js';

/**
 * @typedef {Object} RangeValue
 * @property {string} [start] - Start of range
 * @property {string} [end] - End of range
 * @property {string} [key] - Exact key match
 * @property {string} [keyPrefix] - Key prefix match
 */

/**
 * @typedef {Object} RangeFieldConfig
 * @property {string} [rangeKeyName] - Name of the range key for display
 * @property {'range'|'key'|'prefix'|'all'} [mode] - Range input mode
 * @property {boolean} [required] - Whether field is required
 * @property {boolean} [disabled] - Whether field is disabled
 */

/**
 * @typedef {Object} RangeFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {RangeValue} value - Current range value
 * @property {function} onChange - Callback when value changes (value) => void
 * @property {string} [error] - Error message to display
 * @property {string} [helpText] - Help text to display
 * @property {RangeFieldConfig} [config] - Configuration options
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Simplified range input field component
 *
 * @param {RangeFieldProps} props
 * @returns {JSX.Element}
 */
function RangeField({
  name,
  label,
  value = {},
  onChange,
  error,
  helpText,
  config = {},
  className = ''
}) {
  // Validate and process configuration
  const validatedConfig = validateRangeConfig(config);
  const { mode, rangeKeyName, required, disabled } = validatedConfig;
  
  // Get mode configuration
  const modeConfig = getRangeModeConfig(mode);
  
  // Use range mode hook for state management
  const rangeMode = useRangeMode(value, onChange, modeConfig.availableModes);
  const { state, actions } = rangeMode;
  
  // Get mode labels and visibility
  const modeLabels = getModeLabels();
  const visibleFields = getVisibleFields(mode, state.activeMode);
  
  // Generate help text
  const generatedHelpText = generateRangeHelpText(mode, rangeKeyName, helpText);

  return (
    <FieldWrapper
      label={label}
      name={name}
      required={required}
      error={error}
      helpText={generatedHelpText}
      className={className}
    >
      <div className={getRangeFieldContainerStyles()}>
        {/* Range Key Display */}
        <div className="mb-3">
          <span className={getRangeKeyDisplayStyles()}>
            Range Key: {rangeKeyName}
          </span>
        </div>

        {/* Mode Selection */}
        {modeConfig.showModeSelector && (
          <div className={getModeSelectorStyles()}>
            {modeConfig.availableModes.map((modeKey) => (
              <button
                key={modeKey}
                type="button"
                onClick={() => actions.changeMode(modeKey)}
                className={getModeButtonStyles(state.activeMode === modeKey)}
              >
                {modeLabels[modeKey]}
              </button>
            ))}
          </div>
        )}

        {/* Input Fields */}
        <div className={getInputGridStyles()}>
          {/* Range Fields */}
          {visibleFields.showRange && (
            <>
              <TextField
                name={`${name}-start`}
                label="Start Key"
                value={state.value.start || ''}
                onChange={(newValue) => actions.updateValue('start', newValue)}
                placeholder="Start key"
                disabled={disabled}
                className="col-span-1"
              />
              <TextField
                name={`${name}-end`}
                label="End Key"
                value={state.value.end || ''}
                onChange={(newValue) => actions.updateValue('end', newValue)}
                placeholder="End key"
                disabled={disabled}
                className="col-span-1"
              />
            </>
          )}

          {/* Exact Key Field */}
          {visibleFields.showKey && (
            <TextField
              name={`${name}-key`}
              label="Exact Key"
              value={state.value.key || ''}
              onChange={(newValue) => actions.updateValue('key', newValue)}
              placeholder={`Exact ${rangeKeyName} to match`}
              disabled={disabled}
              className="col-span-1"
            />
          )}

          {/* Prefix Field */}
          {visibleFields.showPrefix && (
            <TextField
              name={`${name}-prefix`}
              label="Key Prefix"
              value={state.value.keyPrefix || ''}
              onChange={(newValue) => actions.updateValue('keyPrefix', newValue)}
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