import { useState, useEffect } from 'react'
import { CheckCircleIcon, TrashIcon } from '@heroicons/react/24/solid'
import { systemClient } from '../api/clients/systemClient'
import { ingestionClient } from '../api/clients'

function StatusSection() {
  const [showConfirmDialog, setShowConfirmDialog] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)
  const [activeJobs, setActiveJobs] = useState([])

  // Poll for all ingestion progress
  useEffect(() => {
    let mounted = true
    let timeoutId

    const poll = async () => {
      try {
        const response = await ingestionClient.getAllProgress()
        if (mounted && response.success && response.data) {
          // Sort jobs by started_at desc (newest first)
          const sortedJobs = [...response.data].sort((a, b) => 
            new Date(b.started_at) - new Date(a.started_at)
          ).slice(0, 1) // Only show the most recent job
          setActiveJobs(sortedJobs)
        }
        if (mounted) {
          timeoutId = setTimeout(poll, 1000) // Poll every 1s
        }
      } catch (error) {
        console.error('Error polling ingestion progress:', error)
        if (mounted) {
          timeoutId = setTimeout(poll, 2000) // Backoff on error
        }
      }
    }

    poll()

    return () => {
      mounted = false
      if (timeoutId) clearTimeout(timeoutId)
    }
  }, [])



  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)

    try {
      const response = await systemClient.resetDatabase(true)

      if (response.success && response.data) {
        setResetResult({ type: 'success', message: response.data.message })
        setTimeout(() => {
          window.location.reload()
        }, 2000)
      } else {
        setResetResult({ type: 'error', message: response.error || 'Reset failed' })
      }
    } catch (error) {
      setResetResult({ type: 'error', message: `Network error: ${error.message}` })
    } finally {
      setIsResetting(false)
      setShowConfirmDialog(false)
    }
  }

  const ResetConfirmDialog = () => {
    if (!showConfirmDialog) return null

    return (
      <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
        <div className="bg-white rounded-lg p-6 max-w-md w-full mx-4">
          <div className="flex items-center gap-3 mb-4">
            <TrashIcon className="w-6 h-6 text-red-500" />
            <h3 className="text-lg font-semibold text-gray-900">Reset Database</h3>
          </div>

          <div className="mb-6">
            <p className="text-gray-700 mb-2">
              This will permanently delete all data and restart the node:
            </p>
            <ul className="list-disc list-inside text-sm text-gray-600 space-y-1">
              <li>All schemas will be removed</li>
              <li>All stored data will be deleted</li>
              <li>Network connections will be reset</li>
              <li>This action cannot be undone</li>
            </ul>
          </div>

          <div className="flex gap-3 justify-end">
            <button
              onClick={() => setShowConfirmDialog(false)}
              className="px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
              disabled={isResetting}
            >
              Cancel
            </button>
            <button
              onClick={handleResetDatabase}
              disabled={isResetting}
              className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-md hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              {isResetting ? 'Resetting...' : 'Reset Database'}
            </button>
          </div>
        </div>
      </div>
    )
  }

  // Helper to render a job card
  const renderJobCard = (job) => {
    const isCompleted = job.is_complete
    const isFailed = job.is_failed
    const isActive = !isCompleted

    // Calculate duration
    let durationStr = ''
    if (job.started_at) {
      const start = new Date(job.started_at)
      const end = job.completed_at ? new Date(job.completed_at) : new Date()
      const seconds = Math.floor((end - start) / 1000)
      durationStr = seconds > 0 ? `${seconds}s` : 'Just started'
    }

    // Determine status color/text
    const statusColor = isFailed ? 'red' : (isCompleted ? 'green' : 'blue')
    const statusText = isFailed ? 'Failed' : (isCompleted ? 'Complete' : 'Active')
    
    // Construct metrics
    const metrics = []
    if (durationStr) metrics.push(durationStr)
    if (job.results?.mutations_executed !== undefined) {
      metrics.push(`${job.results.mutations_executed} items`)
    }

    return (
      <div key={job.id} className={`p-4 rounded-lg border-2 border-${statusColor}-200 bg-${statusColor}-50`}>
        <div className="flex items-center justify-between mb-2">
          <div className="flex items-center gap-2">
            <div className={`w-2.5 h-2.5 rounded-full bg-${statusColor}-500 ${isActive ? 'animate-pulse' : ''}`}></div>
            <h3 className={`font-semibold text-${statusColor}-900`}>
              Ingestion Job
            </h3>
          </div>
          <span className={`text-xs font-medium px-2 py-1 rounded bg-${statusColor}-100 text-${statusColor}-700`}>
            {statusText}
          </span>
        </div>

        <p className={`text-sm text-${statusColor}-700 truncate`} title={job.status_message}>
          {job.status_message}
        </p>

        {metrics.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-2">
            {metrics.map((metric, idx) => (
              <span key={idx} className={`text-xs font-medium px-2 py-1 rounded bg-${statusColor}-100 text-${statusColor}-800`}>
                {metric}
              </span>
            ))}
          </div>
        )}

        {/* Progress Bar for active jobs */}
        {isActive && !isFailed && (
          <div className="mt-3">
            <div className="flex items-center justify-between mb-1">
              <span className={`text-xs font-medium text-${statusColor}-700`}>Progress</span>
              <span className={`text-xs font-semibold text-${statusColor}-900`}>{job.progress_percentage}%</span>
            </div>
            <div className={`w-full bg-${statusColor}-200 rounded-full h-2`}>
              <div
                className={`bg-${statusColor}-600 h-2 rounded-full transition-all duration-300`}
                style={{ width: `${job.progress_percentage}%` }}
              />
            </div>
          </div>
        )}
      </div>
    )
  }



  return (
    <>
      <div className="bg-white rounded-lg shadow-sm p-4 mb-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <CheckCircleIcon className="w-5 h-5 text-green-500" />
            <h2 className="text-lg font-semibold text-gray-900">System Status</h2>
          </div>

          <button
            onClick={() => setShowConfirmDialog(true)}
            className="flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-red-600 border border-red-200 rounded-md hover:bg-red-50 hover:border-red-300 transition-colors"
            disabled={isResetting}
          >
            <TrashIcon className="w-4 h-4" />
            Reset Database
          </button>
        </div>

        {/* Dashboard Grid */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          


          {/* Active Job Cards */}
          {activeJobs.length > 0 && activeJobs.map(job => renderJobCard(job))}
          
          {/* Placeholder if no jobs and indexing is idle? Optional. */}
           {activeJobs.length === 0 && (
            <div className="p-4 rounded-lg border-2 border-dashed border-gray-200 bg-gray-50 flex items-center justify-center text-gray-400 text-sm">
              No active jobs
            </div>
          )}

        </div>

        {resetResult && (
          <div className={`mt-3 p-3 rounded-md text-sm ${resetResult.type === 'success'
              ? 'bg-green-50 text-green-800 border border-green-200'
              : 'bg-red-50 text-red-800 border border-red-200'
            }`}>
            {resetResult.message}
          </div>
        )}
      </div>

      <ResetConfirmDialog />
    </>
  )
}

export default StatusSection