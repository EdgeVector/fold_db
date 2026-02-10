/**
 * Shared SubmitButton component.
 * Renders a consistent submit/action button with loading spinner.
 * Replaces the duplicated {isLoading ? spinner : →label} pattern.
 */

function SubmitButton({
  isLoading = false,
  disabled = false,
  label,
  loadingLabel = 'Processing...',
  onClick,
  type = 'button',
  className = '',
}) {
  const isDisabled = disabled || isLoading

  return (
    <button
      type={type}
      onClick={onClick}
      disabled={isDisabled}
      className={`minimal-btn-secondary px-6 py-2.5 font-medium ${
        isDisabled ? '' : 'minimal-btn'
      } ${className}`}
    >
      {isLoading ? (
        <>
          <span className="minimal-spinner"></span>
          <span>{loadingLabel}</span>
        </>
      ) : (
        <>
          <span>→</span>
          <span>{label}</span>
        </>
      )}
    </button>
  )
}

export default SubmitButton
