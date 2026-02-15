import { useState, useEffect, useCallback } from 'react'
import { ingestionClient } from '../api/clients'
import { getIndexingStatus } from '../api/clients/indexingClient'

/**
 * Compact progress indicator for the header
 * Shows a summary of active jobs with animated progress bar
 * Includes both ingestion/reset jobs AND indexing status
 */
function HeaderProgress() {
  const [jobs, setJobs] = useState([])
  const [indexingStatus, setIndexingStatus] = useState(null)
  const [isLoading, setIsLoading] = useState(true)

  // Poll for progress updates (ingestion + indexing)
  const fetchProgress = useCallback(async () => {
    try {
      // Fetch both ingestion progress and indexing status in parallel
      const [progressResponse, indexingResponse] = await Promise.allSettled([
        ingestionClient.getAllProgress(),
        getIndexingStatus()
      ])

      // Handle ingestion progress
      if (progressResponse.status === 'fulfilled') {
        const response = progressResponse.value
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
      if (indexingResponse.status === 'fulfilled') {
        setIndexingStatus(indexingResponse.value)
      } else {
        setIndexingStatus(null)
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

  // Don't render anything if loading or no activity
  if (isLoading || (activeJobs.length === 0 && !isIndexingActive)) {
    return null
  }

  // Render indicators
  const indicators = []

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

  // Add ingestion/reset job indicators
  if (activeJobs.length > 0) {
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
