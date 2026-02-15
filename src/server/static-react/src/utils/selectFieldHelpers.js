/**
 * SelectField Helper Utilities
 * TASK-009: Additional Simplification - SelectField complexity reduction
 *
 * Extracted from SelectField.jsx to reduce component complexity and improve reusability.
 * These utilities handle option grouping, filtering, and configuration processing.
 */

import { isValueEmpty } from "./rangeSchemaHelpers.js";

/**
 * Groups options by their group property
 * @param {Array} options - Array of select options
 * @returns {Object} Grouped options object
 */
export function groupOptions(options = []) {
  return options.reduce((groups, option) => {
    const group = option.group || "default";
    if (!groups[group]) {
      groups[group] = [];
    }
    groups[group].push(option);
    return groups;
  }, {});
}

/**
 * Filters options based on search term
 * @param {Array} options - Array of select options
 * @param {string} searchTerm - Search term to filter by
 * @returns {Array} Filtered options
 */
export function filterOptions(options = [], searchTerm = "") {
  if (isValueEmpty(searchTerm)) return options;

  const lowerSearchTerm = searchTerm.toLowerCase();
  return options.filter(
    (option) =>
      option.label.toLowerCase().includes(lowerSearchTerm) ||
      option.value.toLowerCase().includes(lowerSearchTerm),
  );
}

/**
 * Processes select field configuration and applies defaults
 * @param {Object} config - Configuration object
 * @returns {Object} Processed configuration with defaults
 */
export function processSelectConfig(config = {}) {
  return {
    placeholder: "Select an option...",
    emptyMessage: "No options available",
    searchable: false,
    required: false,
    disabled: false,
    loading: false,
    showConfirmation: false,
    ...config,
  };
}

/**
 * Determines select field styling based on state
 * @param {Object} styles - Component styles object
 * @param {boolean} hasError - Whether field has error
 * @param {boolean} disabled - Whether field is disabled
 * @param {boolean} loading - Whether field is loading
 * @returns {string} CSS classes string
 */
export function getSelectStyles(
  _styles,
  hasError = false,
  disabled = false,
  loading = false,
) {
  let classes = "select";

  if (hasError) {
    classes += " border-gruvbox-red";
  }

  if (disabled || loading) {
    classes += " opacity-50 cursor-not-allowed";
  }

  return classes;
}

/**
 * Creates ARIA attributes for select field accessibility
 * @param {string} fieldId - Field ID
 * @param {boolean} hasError - Whether field has error
 * @param {string} helpText - Help text content
 * @returns {Object} ARIA attributes object
 */
export function createAriaAttributes(fieldId, hasError = false, helpText = "") {
  const attributes = {
    "aria-invalid": hasError,
  };

  if (hasError) {
    attributes["aria-describedby"] = `${fieldId}-error`;
  } else if (helpText) {
    attributes["aria-describedby"] = `${fieldId}-help`;
  }

  return attributes;
}

/**
 * Validates select field options array
 * @param {Array} options - Options to validate
 * @returns {boolean} True if options are valid
 */
export function validateOptions(options) {
  if (!Array.isArray(options)) return false;

  return options.every(
    (option) =>
      option &&
      typeof option === "object" &&
      typeof option.value !== "undefined" &&
      typeof option.label === "string",
  );
}
