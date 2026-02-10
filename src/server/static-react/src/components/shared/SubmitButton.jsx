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
      className={`btn-secondary btn-lg ${isDisabled ? '' : 'btn-primary'} ${className}`}
    >
      {isLoading ? (
        <>
          <span className="spinner"></span>
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
