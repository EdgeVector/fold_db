/**
 * Form Utilities - Consolidated Form Helper Functions
 * TASK-008: Duplicate Code Detection and Elimination
 *
 * This module consolidates common form-related utilities that were
 * duplicated across multiple form components, providing a single
 * source of truth for form field generation, styling, and validation.
 */

/**
 * Generates a consistent field ID for form fields
 * @param {string} name - Field name
 * @returns {string} Generated field ID
 */
export function generateFieldId(name) {
  return `field-${name}`;
}

/**
 * Checks if a field has an error state
 * @param {string|null|undefined} error - Error message
 * @returns {boolean} True if field has error
 */
export function hasFieldError(error) {
  return Boolean(error);
}

/**
 * Generates input styling classes based on field state
 * @param {Object} options - Styling options
 * @param {boolean} options.hasError - Whether field has error
 * @param {boolean} options.disabled - Whether field is disabled
 * @param {string} [options.additionalClasses=''] - Additional CSS classes
 * @returns {string} Combined CSS classes
 */
export function generateInputStyles({
  hasError,
  disabled,
  additionalClasses = "",
}) {
  const baseStyles = "input";
  const stateStyles = hasError ? "border-gruvbox-red" : "";
  const disabledStyles = disabled ? "opacity-50 cursor-not-allowed" : "";

  return `${baseStyles} ${stateStyles} ${disabledStyles} ${additionalClasses}`.trim();
}

/**
 * Generates select styling classes based on field state
 * @param {Object} options - Styling options
 * @param {boolean} options.hasError - Whether field has error
 * @param {boolean} options.disabled - Whether field is disabled
 * @param {boolean} options.loading - Whether field is loading
 * @param {string} [options.additionalClasses=''] - Additional CSS classes
 * @returns {string} Combined CSS classes
 */
export function generateSelectStyles({
  hasError,
  disabled,
  loading,
  additionalClasses = "",
}) {
  const baseStyles = "select";
  const errorStyles = hasError ? "border-gruvbox-red" : "";
  const disabledStyles =
    disabled || loading ? "opacity-50 cursor-not-allowed" : "";

  return `${baseStyles} ${errorStyles} ${disabledStyles} ${additionalClasses}`.trim();
}

/**
 * Generates ARIA attributes for form fields
 * @param {Object} options - ARIA options
 * @param {string} options.fieldId - Field ID
 * @param {boolean} options.hasError - Whether field has error
 * @param {boolean} options.hasHelp - Whether field has help text
 * @returns {Object} ARIA attributes object
 */
export function generateAriaAttributes({ fieldId, hasError, hasHelp }) {
  const attributes = {
    "aria-invalid": hasError,
  };

  if (hasError) {
    attributes["aria-describedby"] = `${fieldId}-error`;
  } else if (hasHelp) {
    attributes["aria-describedby"] = `${fieldId}-help`;
  }

  return attributes;
}

/**
 * Creates a debounced function for field validation
 * @param {Function} validationFn - Validation function to debounce
 * @param {number} delay - Debounce delay in milliseconds
 * @returns {Function} Debounced validation function
 */
export function createDebouncedValidation(validationFn, delay) {
  let timeoutId;

  return (...args) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => {
      validationFn(...args);
    }, delay);
  };
}

/**
 * Manages internal field value state with external sync
 * @param {*} externalValue - Value from parent component
 * @param {Function} onChange - Change handler from parent
 * @param {Function} transform - Optional value transformation function
 * @returns {Object} State management object
 */
export function createFieldValueManager(
  externalValue,
  onChange,
  _transform = (val) => val,
) {
  // This would typically be used within a component with useState/useEffect
  // Providing the pattern for consistent implementation
  return {
    // Pattern for components to implement:
    // const [internalValue, setInternalValue] = useState(transform(externalValue));
    //
    // useEffect(() => {
    //   setInternalValue(transform(externalValue));
    // }, [externalValue]);
    //
    // const handleChange = (newValue) => {
    //   setInternalValue(newValue);
    //   onChange(transform(newValue));
    // };
    pattern: "Use useState and useEffect with transform function",
  };
}

/**
 * Common field props interface for consistent field component APIs
 * @typedef {Object} CommonFieldProps
 * @property {string} name - Field name for form handling
 * @property {string} label - Field label text
 * @property {*} value - Current field value
 * @property {function} onChange - Change handler (value) => void
 * @property {boolean} [required=false] - Whether field is required
 * @property {boolean} [disabled=false] - Whether field is disabled
 * @property {string} [error] - Error message to display
 * @property {string} [placeholder] - Placeholder text
 * @property {string} [helpText] - Help text to display
 * @property {string} [className=''] - Additional CSS classes
 */

/**
 * Validates that a component implements the common field interface
 * @param {Object} props - Component props to validate
 * @returns {boolean} True if props conform to interface
 */
export function validateFieldProps(props) {
  const required = ["name", "label", "value", "onChange"];
  return required.every((prop) => prop in props);
}

/**
 * Common loading spinner component markup
 * @param {Object} options - Spinner options
 * @param {string} [options.size='sm'] - Spinner size (sm, md, lg)
 * @param {string} [options.color='primary'] - Spinner color
 * @returns {string} Spinner HTML class string
 */
export function getLoadingSpinnerClasses({
  size = "sm",
  color = "primary",
} = {}) {
  const sizeClasses = {
    sm: "h-3 w-3",
    md: "h-4 w-4",
    lg: "h-5 w-5",
  };

  const colorClasses = {
    primary: "border-primary border-t-transparent",
    gray: "border-border border-t-transparent",
    white: "border-white border-t-transparent",
  };

  return `animate-spin ${sizeClasses[size]} border-2 ${colorClasses[color]} rounded-full`;
}
