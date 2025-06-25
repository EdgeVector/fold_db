/**
 * useRangeMode Hook
 * TASK-009: Additional Simplification - Extracted from RangeField complexity
 * 
 * Custom hook for managing range field mode state and validation logic.
 * This extraction reduces RangeField component complexity and improves reusability.
 */

import { useState, useCallback } from 'react';

/**
 * @typedef {Object} RangeValue
 * @property {string} [start] - Start of range
 * @property {string} [end] - End of range
 * @property {string} [key] - Exact key match
 * @property {string} [keyPrefix] - Key prefix match
 */

/**
 * @typedef {Object} RangeModeState
 * @property {('range'|'key'|'prefix')} activeMode - Current active mode
 * @property {RangeValue} value - Current range value
 */

/**
 * @typedef {Object} RangeModeActions
 * @property {Function} changeMode - Change the active mode and clear conflicting values
 * @property {Function} updateValue - Update a specific field in the range value
 * @property {Function} clearValue - Clear all values
 * @property {Function} setValue - Set the entire value object
 */

/**
 * @typedef {Object} UseRangeModeResult
 * @property {RangeModeState} state - Current state
 * @property {RangeModeActions} actions - Available actions
 * @property {Function} getAvailableModes - Get available modes based on configuration
 * @property {Function} isValidMode - Check if mode is valid for current configuration
 */

/**
 * Determines initial mode based on existing value
 * @param {RangeValue} value - Current value
 * @returns {string} Initial mode
 */
function determineInitialMode(value = {}) {
  if (value.start || value.end) return 'range';
  if (value.key) return 'key';
  if (value.keyPrefix) return 'prefix';
  return 'range'; // Default to range mode
}

/**
 * Clears conflicting values when switching modes
 * @param {RangeValue} value - Current value
 * @param {string} newMode - New mode being set
 * @param {string} field - Field being updated
 * @returns {RangeValue} Cleaned value object
 */
function clearConflictingValues(value, newMode, field) {
  const updatedValue = { ...value };
  
  if (newMode === 'range' || field === 'start' || field === 'end') {
    delete updatedValue.key;
    delete updatedValue.keyPrefix;
  } else if (newMode === 'key' || field === 'key') {
    delete updatedValue.start;
    delete updatedValue.end;
    delete updatedValue.keyPrefix;
  } else if (newMode === 'prefix' || field === 'keyPrefix') {
    delete updatedValue.start;
    delete updatedValue.end;
    delete updatedValue.key;
  }
  
  return updatedValue;
}

/**
 * Custom hook for managing range field mode functionality
 * 
 * Provides state management for different range field modes (range, key, prefix)
 * and handles value updates with automatic conflict resolution.
 * 
 * @param {RangeValue} initialValue - Initial range value
 * @param {Function} onChange - Callback when value changes
 * @param {Array<string>} allowedModes - Array of allowed modes (default: ['range', 'key', 'prefix'])
 * @returns {UseRangeModeResult} Hook result with state and actions
 * 
 * @example
 * ```jsx
 * function RangeInput({ value, onChange }) {
 *   const { state, actions, getAvailableModes } = useRangeMode(
 *     value, 
 *     onChange, 
 *     ['range', 'key']
 *   );
 * 
 *   const modes = getAvailableModes();
 * 
 *   return (
 *     <div>
 *       {modes.map(mode => (
 *         <button 
 *           key={mode}
 *           onClick={() => actions.changeMode(mode)}
 *           className={state.activeMode === mode ? 'active' : ''}
 *         >
 *           {mode}
 *         </button>
 *       ))}
 * 
 *       {state.activeMode === 'range' && (
 *         <>
 *           <input 
 *             value={state.value.start || ''}
 *             onChange={(e) => actions.updateValue('start', e.target.value)}
 *           />
 *           <input 
 *             value={state.value.end || ''}
 *             onChange={(e) => actions.updateValue('end', e.target.value)}
 *           />
 *         </>
 *       )}
 *     </div>
 *   );
 * }
 * ```
 */
export function useRangeMode(
  initialValue = {},
  onChange,
  allowedModes = ['range', 'key', 'prefix']
) {
  const [activeMode, setActiveMode] = useState(() => 
    determineInitialMode(initialValue)
  );
  const [value, setValue] = useState(initialValue);

  /**
   * Change the active mode and clear conflicting values
   */
  const changeMode = useCallback((newMode) => {
    if (!allowedModes.includes(newMode)) return;
    
    setActiveMode(newMode);
    
    // Clear all values when changing modes to prevent conflicts
    const clearedValue = {};
    setValue(clearedValue);
    
    if (onChange) {
      onChange(clearedValue);
    }
  }, [allowedModes, onChange]);

  /**
   * Update a specific field in the range value
   */
  const updateValue = useCallback((field, newValue) => {
    const updatedValue = clearConflictingValues(value, activeMode, field);
    updatedValue[field] = newValue;
    
    setValue(updatedValue);
    
    if (onChange) {
      onChange(updatedValue);
    }
  }, [value, activeMode, onChange]);

  /**
   * Clear all values
   */
  const clearValue = useCallback(() => {
    const clearedValue = {};
    setValue(clearedValue);
    
    if (onChange) {
      onChange(clearedValue);
    }
  }, [onChange]);

  /**
   * Set the entire value object
   */
  const setEntireValue = useCallback((newValue) => {
    setValue(newValue);
    
    // Update active mode based on new value
    const newMode = determineInitialMode(newValue);
    setActiveMode(newMode);
    
    if (onChange) {
      onChange(newValue);
    }
  }, [onChange]);

  /**
   * Get available modes based on configuration
   */
  const getAvailableModes = useCallback(() => {
    return allowedModes;
  }, [allowedModes]);

  /**
   * Check if mode is valid for current configuration
   */
  const isValidMode = useCallback((mode) => {
    return allowedModes.includes(mode);
  }, [allowedModes]);

  const state = {
    activeMode,
    value
  };

  const actions = {
    changeMode,
    updateValue,
    clearValue,
    setValue: setEntireValue
  };

  return {
    state,
    actions,
    getAvailableModes,
    isValidMode
  };
}

export default useRangeMode;