/**
 * RangeField Helper Utilities
 * TASK-009: Additional Simplification - RangeField complexity reduction
 * 
 * Extracted from RangeField.jsx to reduce component complexity and improve reusability.
 * These utilities handle help text generation, mode configuration, and range field styling.
 */

import { HELP_TEXT } from '../constants/ui.js';

/**
 * Generates help text for range field based on mode and range key name
 * @param {string} mode - Current mode ('all', 'range', 'key', 'prefix')
 * @param {string} rangeKeyName - Name of the range key for display
 * @param {string} customHelpText - Custom help text to override default
 * @returns {React.ReactNode|string} Generated help text
 */
export function generateRangeHelpText(mode = 'all', rangeKeyName = 'key', customHelpText = '') {
  if (customHelpText) return customHelpText;

  if (mode !== 'all') return null;

  return `Filter by ${rangeKeyName}: use exact ${rangeKeyName}, ${rangeKeyName} range, or ${rangeKeyName} prefix. Leave empty to return all records.`;
}

/**
 * Gets the mode configuration for range field
 * @param {string} mode - Mode type ('all', 'range', 'key', 'prefix')
 * @returns {Object} Mode configuration
 */
export function getRangeModeConfig(mode = 'all') {
  const configs = {
    all: {
      showModeSelector: true,
      availableModes: ['range', 'key', 'prefix'],
      defaultMode: 'range'
    },
    range: {
      showModeSelector: false,
      availableModes: ['range'],
      defaultMode: 'range'
    },
    key: {
      showModeSelector: false,
      availableModes: ['key'],
      defaultMode: 'key'
    },
    prefix: {
      showModeSelector: false,
      availableModes: ['prefix'],
      defaultMode: 'prefix'
    }
  };
  
  return configs[mode] || configs.all;
}

/**
 * Gets mode button styling
 * @param {boolean} isActive - Whether this mode is currently active
 * @returns {string} CSS classes for mode button
 */
export function getModeButtonStyles(isActive = false) {
  const baseStyles = 'px-3 py-1 text-xs transition-colors duration-200';
  
  if (isActive) {
    return `${baseStyles} bg-gruvbox-orange text-surface`;
  }
  
  return `${baseStyles} bg-gruvbox-elevated text-secondary hover:bg-gruvbox-hover`;
}

/**
 * Gets the mode display labels
 * @returns {Object} Mode labels mapping
 */
export function getModeLabels() {
  return {
    range: 'Key Range',
    key: 'Exact Key',
    prefix: 'Key Prefix'
  };
}

/**
 * Determines which fields should be visible based on mode and configuration
 * @param {string} mode - Range field mode ('all', 'range', 'key', 'prefix')
 * @param {string} activeMode - Currently active mode when mode is 'all'
 * @returns {Object} Object indicating which field types are visible
 */
export function getVisibleFields(mode, activeMode) {
  if (mode === 'all') {
    return {
      showRange: activeMode === 'range',
      showKey: activeMode === 'key',
      showPrefix: activeMode === 'prefix'
    };
  }
  
  return {
    showRange: mode === 'range',
    showKey: mode === 'key',
    showPrefix: mode === 'prefix'
  };
}

/**
 * Validates range field configuration
 * @param {Object} config - Range field configuration
 * @returns {Object} Validated and normalized configuration
 */
export function validateRangeConfig(config = {}) {
  const {
    mode = 'all',
    rangeKeyName = 'key',
    required = false,
    disabled = false,
    className = ''
  } = config;
  
  // Validate mode
  const validModes = ['all', 'range', 'key', 'prefix'];
  const validatedMode = validModes.includes(mode) ? mode : 'all';
  
  return {
    mode: validatedMode,
    rangeKeyName: String(rangeKeyName),
    required: Boolean(required),
    disabled: Boolean(disabled),
    className: String(className)
  };
}

/**
 * Gets the range field container styling
 * @returns {string} CSS classes for range field container
 */
export function getRangeFieldContainerStyles() {
  return 'bg-gruvbox-elevated border border-border p-4 space-y-4';
}

/**
 * Gets the range key display styles
 * @returns {string} CSS classes for range key display
 */
export function getRangeKeyDisplayStyles() {
  return 'text-sm font-medium text-primary';
}

/**
 * Gets the mode selector container styles
 * @returns {string} CSS classes for mode selector container
 */
export function getModeSelectorStyles() {
  return 'flex space-x-4 mb-4';
}

/**
 * Gets the input grid styles
 * @returns {string} CSS classes for input grid container
 */
export function getInputGridStyles() {
  return 'grid grid-cols-1 md:grid-cols-3 gap-4';
}