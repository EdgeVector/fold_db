/**
 * FieldWrapper Component
 * Provides consistent layout for form fields with labels, help text, and error messages
 * Part of TASK-002: Component Extraction and Modularization
 */


/**
 * @typedef {Object} FieldWrapperProps
 * @property {string} label - Field label text
 * @property {string} [name] - Field name for accessibility
 * @property {boolean} [required] - Whether field is required
 * @property {string} [error] - Error message to display
 * @property {string} [helpText] - Help text to display below field
 * @property {React.ReactNode} children - Form field element(s)
 * @property {string} [className] - Additional CSS classes
 */

/**
 * Wrapper component for form fields providing consistent layout and styling
 * 
 * @param {FieldWrapperProps} props
 * @returns {JSX.Element}
 */
function FieldWrapper({
  label,
  name,
  required = false,
  error,
  helpText,
  children,
  className = ''
}) {
  const fieldId = name ? `field-${name}` : `field-${Math.random().toString(36).substr(2, 9)}`;
  const hasError = Boolean(error);

  return (
    <div className={`space-y-2 ${className}`}>
      {/* Label */}
      <label 
        htmlFor={fieldId}
        className="label"
      >
        {label}
        {required && (
          <span className="ml-1 text-gruvbox-red" aria-label="required">
            *
          </span>
        )}
      </label>

      {/* Form Field */}
      <div className="relative">
        {children}
      </div>

      {/* Error Message */}
      {hasError && (
        <p 
          className="text-sm text-gruvbox-red"
          role="alert"
          aria-live="polite"
        >
          {error}
        </p>
      )}

      {/* Help Text */}
      {helpText && !hasError && (
        <p className="text-xs text-secondary">
          {helpText}
        </p>
      )}
    </div>
  );
}

export default FieldWrapper;