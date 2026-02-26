import { useState, useEffect } from 'react'
import IngestionReport from '../../IngestionReport'
import { fmtCost } from '../../../utils/formatCost'

/** Clickable "(N failed)" badge that expands to show failed file details */
function FailedFilesBadge({ count, files }) {
  const [expanded, setExpanded] = useState(false)
  if (count === 0) return null
  return (
    <>
      <button
        onClick={() => setExpanded(prev => !prev)}
        className="text-gruvbox-red underline cursor-pointer hover:opacity-80"
      >
        ({count} failed)
      </button>
      {expanded && files?.length > 0 && (
        <div className="mt-2 bg-surface-secondary border border-gruvbox-red/30 rounded p-2 text-xs space-y-1 max-h-40 overflow-y-auto">
          {files.map((f, i) => (
            <div key={i} className="flex flex-col">
              <span className="font-mono text-primary">{f.name}</span>
              <span className="text-gruvbox-red ml-2">{f.error}</span>
            </div>
          ))}
        </div>
      )}
    </>
  )
}

/**
 * Displays batch ingestion progress: running, paused, completed, cancelled, failed states.
 *
 * @param {Object} props
 * @param {Object|null} props.batchStatus
 * @param {Object|null} props.batchReport
 * @param {Function} props.onResume - Called with new spend limit (number)
 * @param {Function} props.onCancel
 * @param {Function} props.onBack
 * @param {Function} props.onDismissReport
 * @param {boolean} props.isIngesting - True while waiting for first batch poll
 */
export default function BatchProgressView({
  batchStatus,
  batchReport,
  onResume,
  onCancel,
  onBack,
  onDismissReport,
  isIngesting,
}) {
  const [newLimit, setNewLimit] = useState('')

  // Pre-fill new limit when paused
  useEffect(() => {
    if (batchStatus?.status === 'Paused') {
      const suggested = batchStatus.accumulated_cost + batchStatus.estimated_remaining_cost
      setNewLimit(suggested.toFixed(2))
    }
  }, [batchStatus?.status, batchStatus?.accumulated_cost, batchStatus?.estimated_remaining_cost])

  const isRunning = batchStatus?.status === 'Running'
  const isPaused = batchStatus?.status === 'Paused'
  const isCompleted = batchStatus?.status === 'Completed'
  const isCancelled = batchStatus?.status === 'Cancelled'
  const isFailed = batchStatus?.status === 'Failed'
  const isDone = isCompleted || isCancelled || isFailed

  const handleResume = () => {
    const limit = parseFloat(newLimit)
    if (isNaN(limit) || limit <= 0) return
    onResume(limit)
  }

  // Running
  if (isRunning && batchStatus) {
    return (
      <div className="space-y-3">
        <p className="text-sm font-medium">Ingesting files...</p>
        <div className="w-full bg-border rounded-full h-2 overflow-hidden">
          <div
            className="h-full bg-primary transition-all duration-300"
            style={{ width: `${batchStatus.files_total > 0 ? Math.round((batchStatus.files_completed / batchStatus.files_total) * 100) : 0}%` }}
          />
        </div>
        <div className="flex items-center justify-between text-sm text-secondary">
          <span>{batchStatus.files_completed}/{batchStatus.files_total} files {batchStatus.files_failed > 0 && <FailedFilesBadge count={batchStatus.files_failed} files={batchStatus.failed_files} />}</span>
          {batchStatus.is_local_provider ? (
            <span>Free (local)</span>
          ) : (
            <span>{fmtCost(batchStatus.accumulated_cost)} spent{batchStatus.spend_limit != null ? ` / ${fmtCost(batchStatus.spend_limit)} limit` : ''}</span>
          )}
        </div>
        {/* Current file sub-progress */}
        {batchStatus.current_file_name && (
          <div className="bg-surface-secondary border border-border rounded p-2 space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-xs font-mono text-primary truncate max-w-[60%]" title={batchStatus.current_file_name}>{batchStatus.current_file_name}</span>
              <span className="text-xs text-secondary">{batchStatus.current_file_progress ?? 0}%</span>
            </div>
            <div className="w-full bg-border rounded-full h-1 overflow-hidden">
              <div
                className="h-full bg-gruvbox-blue transition-all duration-300"
                style={{ width: `${batchStatus.current_file_progress ?? 0}%` }}
              />
            </div>
            {batchStatus.current_file_step && (
              <p className="text-xs text-secondary truncate">{batchStatus.current_file_step}</p>
            )}
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button onClick={onCancel} className="btn-secondary">Cancel</button>
          <button onClick={onBack} className="btn-secondary">Scan Another</button>
        </div>
      </div>
    )
  }

  // Paused
  if (isPaused && batchStatus) {
    return (
      <div className="space-y-3">
        <p className="text-sm font-medium text-gruvbox-yellow">Paused -- spend limit reached</p>
        <div className="w-full bg-border rounded-full h-2 overflow-hidden">
          <div
            className="h-full bg-gruvbox-yellow transition-all duration-300"
            style={{ width: `${batchStatus.files_total > 0 ? Math.round((batchStatus.files_completed / batchStatus.files_total) * 100) : 0}%` }}
          />
        </div>
        <div className="flex items-center justify-between text-sm text-secondary">
          <span>{batchStatus.files_completed}/{batchStatus.files_total} files</span>
          <span>{fmtCost(batchStatus.accumulated_cost)} spent / {fmtCost(batchStatus.spend_limit)} limit</span>
        </div>
        <p className="text-sm text-secondary">
          {batchStatus.files_remaining} files remaining (~{fmtCost(batchStatus.estimated_remaining_cost)} to finish)
        </p>
        <div className="flex items-center gap-3">
          <label className="flex items-center gap-2 text-sm text-secondary">
            New limit:
            <input
              type="text"
              value={newLimit}
              onChange={(e) => setNewLimit(e.target.value)}
              className="input w-24 text-sm"
            />
          </label>
          <button onClick={handleResume} className="btn-primary">Resume</button>
          <button onClick={onCancel} className="btn-secondary">Stop</button>
        </div>
      </div>
    )
  }

  // Completed / Cancelled / Failed
  if (isDone && batchStatus) {
    return (
      <div className="space-y-3">
        <p className="text-sm font-medium">
          {isCompleted && 'Ingestion complete'}
          {isCancelled && 'Ingestion cancelled'}
          {isFailed && 'Ingestion failed'}
        </p>
        <div className="text-sm text-secondary">
          <span>
            {batchStatus.files_completed} files ingested
            {' '}{batchStatus.files_failed > 0 && <FailedFilesBadge count={batchStatus.files_failed} files={batchStatus.failed_files} />}
            {batchStatus.is_local_provider ? ' · Free (local)' : ` · ${fmtCost(batchStatus.accumulated_cost)} spent`}
          </span>
        </div>
        {batchReport && (
          <IngestionReport
            ingestionResult={batchReport}
            onDismiss={onDismissReport}
          />
        )}
        <div className="flex justify-end">
          <button onClick={onBack} className="btn-secondary">Scan Another</button>
        </div>
      </div>
    )
  }

  // Waiting for first batch status poll
  if (isIngesting) {
    return (
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2 text-sm text-secondary">
          <span className="spinner" /> Starting batch...
        </div>
        <button onClick={onBack} className="btn-secondary text-sm">Cancel</button>
      </div>
    )
  }

  return null
}
