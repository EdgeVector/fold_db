import { useState, useEffect, useCallback, useRef } from 'react'
import { ingestionClient } from '../api/clients'
import { getIndexingStatus } from '../api/clients/indexingClient'
import { fmtCost } from '../utils/formatCost'

/**
 * Compact progress indicator for the header
 * Shows a summary of active jobs with animated progress bar
 * Includes both ingestion/reset jobs AND indexing status
 * Also shows batch-level cost tracking when a smart folder batch is active
 */
function HeaderProgress() {
  const [jobs, setJobs] = useState([])
  const [indexingStatus, setIndexingStatus] = useState(null)
  const [batchInfo, setBatchInfo] = useState(null)
  const [isLoading, setIsLoading] = useState(true)
  const dismissTimeoutRef = useRef(null)

  useEffect(() => {
    return () => { if (dismissTimeoutRef.current) clearTimeout(dismissTimeoutRef.current) }
  }, [])

  // Poll for progress updates (ingestion + indexing + batch status)
  const fetchProgress = useCallback(async () => {
    try {
      // Build promises to fetch in parallel
      const promises = [
        ingestionClient.getAllProgress(),
        getIndexingStatus()
      ]

      // Also fetch batch status if one is active
      const activeBatchId = localStorage.getItem('activeBatchId')
      if (activeBatchId) {
        promises.push(ingestionClient.getBatchStatus(activeBatchId))
      }

      const results = await Promise.allSettled(promises)

      // Handle ingestion progress
      if (results[0].status === 'fulfilled') {
        const response = results[0].value
        const progressData = response.data?.progress || response.data || response.progress || []
        if (Array.isArray(progressData)) {
          setJobs(progressData)
        } else {
          setJobs([])
        }
      } else {
        setJobs([])
      }

      // Handle indexing status
      if (results[1].status === 'fulfilled') {
        setIndexingStatus(results[1].value)
      } else {
        setIndexingStatus(null)
      }

      // Handle batch status
      if (results.length > 2 && results[2].status === 'fulfilled') {
        const batchResp = results[2].value
        if (batchResp.success && batchResp.data) {
          setBatchInfo(batchResp.data)
          const s = batchResp.data.status
          if (s === 'Completed' || s === 'Cancelled' || s === 'Failed') {
            // Clear after a short delay so the final state shows briefly
            if (dismissTimeoutRef.current) clearTimeout(dismissTimeoutRef.current)
            dismissTimeoutRef.current = setTimeout(() => setBatchInfo(null), 5000)
          }
        } else {
          setBatchInfo(null)
        }
      } else if (!activeBatchId) {
        setBatchInfo(null)
      }
    } catch (error) {
      console.error('Failed to fetch progress:', error)
      setJobs([])
      setIndexingStatus(null)
    } finally {
      setIsLoading(false)
    }
  }, [])

  useEffect(() => {
    // Initial fetch
    fetchProgress()

    // Set up polling - poll every 2 seconds
    const intervalId = setInterval(fetchProgress, 2000)

    return () => clearInterval(intervalId)
  }, [fetchProgress])

  // Filter to only active jobs (in-progress, not complete/failed)
  const activeJobs = jobs.filter(j => !j.is_complete && !j.is_failed)

  // Check if indexing is active
  const isIndexingActive = indexingStatus?.state === 'Indexing'

  // Check if we have a batch to show
  const hasBatch = batchInfo && (batchInfo.status === 'Running' || batchInfo.status === 'Paused')

  // Don't render anything if loading or no activity
  if (isLoading || (activeJobs.length === 0 && !isIndexingActive && !hasBatch)) {
    return null
  }

  // Render indicators
  const indicators = []

  // Add batch cost indicator (takes priority display)
  if (hasBatch) {
    const isPaused = batchInfo.status === 'Paused'
    const pct = batchInfo.files_total > 0 ? Math.round((batchInfo.files_completed / batchInfo.files_total) * 100) : 0

    const dotColor = isPaused ? 'bg-gruvbox-yellow' : 'bg-gruvbox-blue'
    const dotAnim = isPaused ? '' : 'animate-pulse'
    const textClass = isPaused ? 'text-gruvbox-yellow' : 'text-gruvbox-blue'
    const barColor = isPaused ? 'bg-gruvbox-yellow' : 'bg-primary'

    const fileLabel = batchInfo.current_file_name ? ` ${batchInfo.current_file_name}` : ''
    const statusText = isPaused
      ? `paused ${fmtCost(batchInfo.accumulated_cost)}/${fmtCost(batchInfo.spend_limit)} limit`
      : `ingesting${fileLabel} ${pct}% ${fmtCost(batchInfo.accumulated_cost)}${batchInfo.spend_limit != null ? `/${fmtCost(batchInfo.spend_limit)}` : ''}`

    indicators.push(
      <div key="batch" className="flex items-center gap-2 px-3 py-1.5 bg-surface-secondary border border-border">
        <div className={`w-2 h-2 rounded-full ${dotColor} ${dotAnim}`} />
        <span className={`text-xs font-mono ${textClass}`}>{statusText}</span>
        <div className="w-16 h-1 bg-border rounded-full overflow-hidden">
          <div
            className={`h-full ${barColor} transition-all duration-300`}
            style={{ width: `${pct}%` }}
          />
        </div>
      </div>
    )
  }

  // Add indexing indicator if active
  if (isIndexingActive) {
    const opsQueued = indexingStatus?.operations_queued || 0
    const opsInProgress = indexingStatus?.operations_in_progress || 0
    indicators.push(
      <div key="indexing" className="flex items-center gap-2 px-3 py-1.5 bg-surface-secondary border border-border">
        {/* Animated spinner */}
        <div className="w-2 h-2 bg-gruvbox-blue rounded-full animate-pulse" />
        {/* Status text */}
        <span className="text-xs font-mono text-secondary">
          indexing {opsInProgress > 0 ? `(${opsInProgress})` : ''}{opsQueued > 0 ? ` +${opsQueued}` : ''}
        </span>
      </div>
    )
  }

  // Add ingestion/reset job indicators (only if no batch indicator already showing)
  if (activeJobs.length > 0 && !hasBatch) {
    // Calculate aggregate progress if multiple jobs
    const totalProgress = activeJobs.reduce((sum, job) => sum + (job.progress_percentage || 0), 0)
    const avgProgress = Math.round(totalProgress / activeJobs.length)

    // Get job type label
    const getJobLabel = (job) => {
      if (job.job_type === 'database_reset') return 'reset'
      if (job.job_type === 'indexing') return 'indexing'
      return 'ingesting'
    }

    // For single job, show its status; for multiple, show count
    const statusText = activeJobs.length === 1
      ? `${getJobLabel(activeJobs[0])} ${avgProgress}%`
      : `${activeJobs.length} jobs ${avgProgress}%`

    // Determine color based on primary job type
    const primaryJob = activeJobs[0]
    const isReset = primaryJob?.job_type === 'database_reset'
    const isJobIndexing = primaryJob?.job_type === 'indexing'

    const dotColor = isReset
      ? 'bg-gruvbox-red'
      : isJobIndexing
        ? 'bg-gruvbox-blue'
        : 'bg-gruvbox-blue'

    const textClass = isReset
      ? 'text-gruvbox-red'
      : isJobIndexing
        ? 'text-secondary'
        : 'text-gruvbox-blue'

    indicators.push(
      <div key="jobs" className="flex items-center gap-2 px-3 py-1.5 bg-surface-secondary border border-border">
        {/* Animated spinner */}
        <div className={`w-2 h-2 rounded-full animate-pulse ${dotColor}`} />

        {/* Status text */}
        <span className={`text-xs font-mono ${textClass}`}>
          {statusText}
        </span>

        {/* Mini progress bar */}
        <div className="w-16 h-1 bg-border rounded-full overflow-hidden">
          <div
            className="h-full bg-primary transition-all duration-300"
            style={{ width: `${avgProgress}%` }}
          />
        </div>
      </div>
    )
  }

  return <>{indicators}</>
}

export default HeaderProgress
