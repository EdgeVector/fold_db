import { useState, useEffect, useCallback } from 'react'
import { CheckCircleIcon, TrashIcon, ArrowPathIcon, ClockIcon, XCircleIcon } from '@heroicons/react/24/solid'
import { systemClient } from '../api/clients/systemClient'
import { ingestionClient } from '../api/clients'

function StatusSection() {
  const [showConfirmDialog, setShowConfirmDialog] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)
  const [jobs, setJobs] = useState([])
  const [isLoadingJobs, setIsLoadingJobs] = useState(true)

  // Poll for progress updates
  const fetchProgress = useCallback(async () => {
    try {
      const response = await ingestionClient.getAllProgress()
      // Handle both { success, data } wrapper and { ok, progress } backend format
      const progressData = response.data?.progress || response.data || response.progress || []
      if (Array.isArray(progressData)) {
        setJobs(progressData)
      } else {
        setJobs([])
      }
    } catch (error) {
      console.error('Failed to fetch progress:', error)
      setJobs([])
    } finally {
      setIsLoadingJobs(false)
    }
  }, [])

  useEffect(() => {
    // Initial fetch
    fetchProgress()

    // Set up polling - poll every 2 seconds
    const intervalId = setInterval(fetchProgress, 2000)

    return () => clearInterval(intervalId)
  }, [fetchProgress])

  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)

    try {
      const response = await systemClient.resetDatabase(true)

      // Handle both immediate success (legacy) and async job started (new)
      if (response.success && response.data) {
        if (response.data.job_id) {
          // Async reset started - job will show in progress
          setResetResult({ 
            type: 'success', 
            message: `Reset started (Job: ${response.data.job_id.substring(0, 8)}...). Progress will appear above.`
          })
          // Don't reload - let the user see progress
          setShowConfirmDialog(false)
          setIsResetting(false)
        } else {
          // Legacy immediate completion
          setResetResult({ type: 'success', message: response.data.message })
          setTimeout(() => {
            window.location.reload()
          }, 2000)
        }
      } else {
        setResetResult({ type: 'error', message: response.error || 'Reset failed' })
        setShowConfirmDialog(false)
        setIsResetting(false)
      }
    } catch (error) {
      setResetResult({ type: 'error', message: `Network error: ${error.message}` })
      setShowConfirmDialog(false)
      setIsResetting(false)
    }
  }

  const renderJobCard = (job) => {
    const isIndexing = job.job_type === 'indexing'
    const isDatabaseReset = job.job_type === 'database_reset'
    const jobLabel = isDatabaseReset ? 'Database Reset' : isIndexing ? 'Indexing Job' : 'Ingestion Job'
    
    // Completed jobs get a subtle, grayed-out appearance
    if (job.is_complete) {
      return (
        <div 
          key={job.id} 
          className="card p-3 mb-3 opacity-75"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <CheckCircleIcon className="w-5 h-5 text-tertiary" />
              <span className="font-medium text-secondary">
                {jobLabel}
              </span>
              <span className="badge badge-neutral text-xs">
                Complete
              </span>
            </div>
            <div className="flex items-center gap-1 text-xs text-tertiary">
              <ClockIcon className="w-3 h-3" />
              <span>{new Date(job.started_at).toLocaleTimeString()}</span>
            </div>
          </div>
          <div className="text-xs text-tertiary mt-1">
            {job.status_message || 'Completed successfully'}
          </div>
        </div>
      )
    }

    // Failed jobs show error state
    if (job.is_failed) {
      return (
        <div 
          key={job.id} 
          className="card card-error p-4 mb-3"
        >
          <div className="flex items-center justify-between mb-2">
            <div className="flex items-center gap-2">
              <XCircleIcon className="w-5 h-5 text-red-600" />
              <span className="font-medium text-red-600">
                {jobLabel}
              </span>
              <span className="badge badge-error text-xs">
                Failed
              </span>
            </div>
            <div className="flex items-center gap-1 text-xs text-secondary">
              <ClockIcon className="w-3 h-3" />
              <span>{new Date(job.started_at).toLocaleTimeString()}</span>
            </div>
          </div>
          {job.error_message && (
            <div className="text-xs text-red-600 mt-2">
              Error: {job.error_message}
            </div>
          )}
        </div>
      )
    }

    // In-progress jobs show full progress bar
    const cardClass = isDatabaseReset ? 'card card-error' : isIndexing ? 'card card-info' : 'card card-info'
    const textColor = isIndexing ? 'text-blue-600' : isDatabaseReset ? 'text-red-600' : 'text-blue-600'

    return (
      <div
        key={job.id}
        className={`p-4 ${cardClass} mb-3`}
      >
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <ArrowPathIcon className={`w-5 h-5 ${textColor} animate-spin`} />
            <span className={`font-medium ${textColor}`}>
              {jobLabel}
            </span>
            <span className={`badge ${isDatabaseReset ? 'badge-error' : 'badge-info'}`}>
              In Progress
            </span>
          </div>
          <div className="flex items-center gap-1 text-xs text-secondary">
            <ClockIcon className="w-3 h-3" />
            <span>{new Date(job.started_at).toLocaleTimeString()}</span>
          </div>
        </div>

        {/* Progress bar - black bar showing percentage */}
        <div className="mb-2">
          <div className="flex justify-between text-xs text-secondary mb-1">
            <span>{job.status_message || 'Processing...'}</span>
            <span className="font-medium">{job.progress_percentage || 0}%</span>
          </div>
          <div className="w-full bg-surface-secondary rounded-full h-3">
            <div
              className="h-3 rounded-full transition-all duration-300 bg-black"
              style={{ width: `${job.progress_percentage || 0}%` }}
            />
          </div>
        </div>
      </div>
    )
  }

  const ResetConfirmDialog = () => {
    if (!showConfirmDialog) return null

    return (
      <div className="modal-overlay">
        <div className="modal max-w-md p-6">
          <div className="flex items-center gap-3 mb-4">
            <TrashIcon className="w-6 h-6 text-red-600" />
            <h3 className="text-lg font-semibold text-primary">Reset Database</h3>
          </div>

          <div className="mb-6">
            <p className="text-primary mb-2">
              This will permanently delete all data and restart the node:
            </p>
            <ul className="list-disc list-inside text-sm text-secondary space-y-1">
              <li>All schemas will be removed</li>
              <li>All stored data will be deleted</li>
              <li>Network connections will be reset</li>
              <li>This action cannot be undone</li>
            </ul>
          </div>

          <div className="flex gap-3 justify-end">
            <button
              onClick={() => setShowConfirmDialog(false)}
              className="btn-secondary"
              disabled={isResetting}
            >
              Cancel
            </button>
            <button
              onClick={handleResetDatabase}
              disabled={isResetting}
              className="btn-danger disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isResetting ? 'Resetting...' : 'Reset Database'}
            </button>
          </div>
        </div>
      </div>
    )
  }

  // Filter to show active jobs first, then most recent completed
  const activeJobs = jobs.filter(j => !j.is_complete && !j.is_failed)
  const displayJobs = activeJobs.length > 0 
    ? activeJobs.slice(0, 3) 
    : jobs.filter(j => j.is_complete || j.is_failed).slice(0, 1)

  return (
    <>
      <div className="card p-4 mb-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <CheckCircleIcon className="w-5 h-5 text-green-600" />
            <h2 className="text-lg font-semibold text-primary">System Status</h2>
          </div>

          <button
            onClick={() => setShowConfirmDialog(true)}
            className="btn-danger btn-sm"
            disabled={isResetting}
          >
            <TrashIcon className="w-4 h-4" />
            Reset Database
          </button>
        </div>

        {/* Job Progress Section */}
        {isLoadingJobs ? (
          <div className="card p-4 flex items-center justify-center">
            <ArrowPathIcon className="w-5 h-5 text-tertiary animate-spin mr-2" />
            <span className="text-secondary">Loading status...</span>
          </div>
        ) : displayJobs.length > 0 ? (
          displayJobs.map(job => renderJobCard(job))
        ) : (
          <div className="card card-success p-4">
            <div className="flex items-center gap-2">
              <CheckCircleIcon className="w-5 h-5 text-green-600" />
              <span className="text-green-600 font-medium">No active jobs</span>
            </div>
          </div>
        )}

        {resetResult && (
          <div className={`mt-3 p-3 text-sm ${resetResult.type === 'success' ? 'card card-success text-green-600' : 'card card-error text-red-600'}`}>
            {resetResult.message}
          </div>
        )}
      </div>

      <ResetConfirmDialog />
    </>
  )
}

export default StatusSection