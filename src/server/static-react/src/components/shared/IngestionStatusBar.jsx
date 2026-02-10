/**
 * Shared Ingestion Status Bar component.
 * Displays AI ingestion readiness, provider, and model info.
 * Used by IngestionTab, SmartFolderTab, and FileUploadTab.
 */

function IngestionStatusBar({ ingestionStatus, showConfigHint = false }) {
  if (!ingestionStatus) return null

  const isReady = ingestionStatus.enabled && ingestionStatus.configured

  return (
    <div className="minimal-card minimal-card-accent-success p-3">
      <div className="flex items-center gap-4 text-sm">
        <span className={`minimal-badge ${isReady ? 'minimal-badge-success' : 'minimal-badge-error'}`}>
          {isReady ? 'Ready' : 'Not Configured'}
        </span>
        <span className="text-secondary">{ingestionStatus.provider} · {ingestionStatus.model}</span>
        {showConfigHint && (
          <span className="text-xs text-secondary">Configure AI settings using the Settings button in the header</span>
        )}
      </div>
    </div>
  )
}

export default IngestionStatusBar
