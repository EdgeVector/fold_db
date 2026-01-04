import { useState, useEffect } from 'react'
import { CheckCircleIcon, TrashIcon } from '@heroicons/react/24/solid'
import { systemClient } from '../api/clients/systemClient'
import { ingestionClient } from '../api/clients'
import { useIndexingStatus } from '../api/clients/indexingClient'

function StatusSection() {
  const [showConfirmDialog, setShowConfirmDialog] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)
  const [ingestionProgress, setIngestionProgress] = useState(null)

  const [activeProgressId, setActiveProgressId] = useState(null)

  // Listen for ingestion events
  useEffect(() => {
    const handleIngestionStart = (event) => {
      console.log('🔵 StatusSection: Received ingestion-started event', event.detail)
      setActiveProgressId(event.detail.progressId)
      // Show ingestion immediately, even before first poll
      setIngestionProgress({
        progress_percentage: 0,
        status_message: 'Starting ingestion...',
        is_complete: false
      })
      console.log('🔵 StatusSection: Set initial ingestion progress')
    }

    window.addEventListener('ingestion-started', handleIngestionStart)
    console.log('🔵 StatusSection: Listening for ingestion-started events')
    return () => window.removeEventListener('ingestion-started', handleIngestionStart)
  }, [])

  // Poll ingestion progress
  useEffect(() => {
    if (!activeProgressId) return

    let mounted = true
    let timeoutId

    const poll = async () => {
      try {
        const response = await ingestionClient.getProgress(activeProgressId)
        if (mounted && response.success && response.data) {
          setIngestionProgress(response.data)

          // Keep polling if not complete, stop polling once complete (but keep the data)
          if (!response.data.is_complete) {
            timeoutId = setTimeout(poll, 200) // Poll faster (200ms)
          } else {
            // Leave the completed progress visible - don't clear it
            setActiveProgressId(null) // Stop polling
          }
        } else {
          if (mounted) timeoutId = setTimeout(poll, 200)
        }
      } catch (error) {
        console.error('Error polling ingestion:', error)
        if (mounted) timeoutId = setTimeout(poll, 500)
      }
    }

    // Start immediately
    poll()

    return () => {
      mounted = false
      if (timeoutId) clearTimeout(timeoutId)
    }
  }, [activeProgressId])

  // Use the hook for indexing status with automatic backoff
  // This will poll every 1s when active, and 5s when idle (default behavior of the hook)
  const { status: indexingStatus } = useIndexingStatus(1000);

  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)

    try {
      const response = await systemClient.resetDatabase(true)

      if (response.success && response.data) {
        setResetResult({ type: 'success', message: response.data.message })
        // Refresh the page after a short delay to show the new clean state
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

  // Get ingestion status info
  const getIngestionStatus = () => {
    console.log('🟡 StatusSection getIngestionStatus:', {
      hasProgress: !!ingestionProgress,
      percentage: ingestionProgress?.progress_percentage,
      isComplete: ingestionProgress?.is_complete,
      results: ingestionProgress?.results
    })

    if (ingestionProgress && !ingestionProgress.is_complete) {
      const elapsed = ingestionProgress.started_at
        ? Math.floor((new Date() - new Date(ingestionProgress.started_at)) / 1000)
        : 0

      return {
        state: 'active',
        title: 'Ingesting Data',
        detail: ingestionProgress.status_message,
        percentage: ingestionProgress.progress_percentage,
        metrics: elapsed > 0 ? [`${elapsed}s elapsed`] : [],
        color: 'blue'
      }
    }

    if (ingestionProgress?.is_complete && ingestionProgress?.results) {
      const elapsed = ingestionProgress.started_at && ingestionProgress.completed_at
        ? Math.floor((new Date(ingestionProgress.completed_at) - new Date(ingestionProgress.started_at)) / 1000)
        : 0

      return {
        state: 'completed',
        title: 'Ingestion',
        detail: 'Last ingestion completed',
        metrics: [
          `${ingestionProgress.results.mutations_executed || 0} items ingested`,
          elapsed > 0 ? `${elapsed}s duration` : null
        ].filter(Boolean),
        color: 'green'
      }
    }

    return {
      state: 'idle',
      title: 'Ingestion',
      detail: 'No active ingestion',
      metrics: [],
      color: 'gray'
    }
  }

  // Get indexing status info
  const getIndexingStatusInfo = () => {
    console.log('🟡 StatusSection getIndexingStatus:', {
      indexingState: indexingStatus?.state,
      totalOps: indexingStatus?.total_operations_processed
    })

    if (indexingStatus?.state === 'Indexing') {
      return {
        state: 'active',
        title: 'Background Indexing',
        detail: 'Actively processing index operations',
        metrics: [
          `${indexingStatus.total_operations_processed.toLocaleString()} ops processed`,
          `${indexingStatus.operations_per_second.toFixed(0)} ops/sec`
        ],
        color: 'indigo'
      }
    }

    if (indexingStatus?.total_operations_processed > 0) {
      return {
        state: 'completed',
        title: 'Indexing',
        detail: 'All operations indexed',
        metrics: [`${indexingStatus.total_operations_processed.toLocaleString()} total operations`],
        color: 'green'
      }
    }

    return {
      state: 'idle',
      title: 'Indexing',
      detail: 'No indexing activity',
      metrics: [],
      color: 'gray'
    }
  }

  const ingestionInfo = getIngestionStatus()
  const indexingInfo = getIndexingStatusInfo()

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
          {/* Ingestion Status Card */}
          <div className={`p-4 rounded-lg border-2 ${ingestionInfo.state === 'active'
              ? 'border-blue-200 bg-blue-50'
              : ingestionInfo.state === 'completed'
                ? 'border-green-200 bg-green-50'
                : 'border-gray-200 bg-gray-50'
            }`}>
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <div className={`w-2.5 h-2.5 rounded-full ${ingestionInfo.state === 'active'
                    ? 'bg-blue-500 animate-pulse'
                    : ingestionInfo.state === 'completed'
                      ? 'bg-green-500'
                      : 'bg-gray-400'
                  }`}></div>
                <h3 className={`font-semibold ${ingestionInfo.state === 'active'
                    ? 'text-blue-900'
                    : ingestionInfo.state === 'completed'
                      ? 'text-green-900'
                      : 'text-gray-700'
                  }`}>
                  {ingestionInfo.title}
                </h3>
              </div>
              <span className={`text-xs font-medium px-2 py-1 rounded ${ingestionInfo.state === 'active'
                  ? 'bg-blue-100 text-blue-700'
                  : ingestionInfo.state === 'completed'
                    ? 'bg-green-100 text-green-700'
                    : 'bg-gray-200 text-gray-600'
                }`}>
                {ingestionInfo.state === 'active' ? 'Active' : ingestionInfo.state === 'completed' ? 'Complete' : 'Idle'}
              </span>
            </div>

            <p className={`text-sm ${ingestionInfo.state === 'active'
                ? 'text-blue-700'
                : ingestionInfo.state === 'completed'
                  ? 'text-green-700'
                  : 'text-gray-500'
              }`}>
              {ingestionInfo.detail}
            </p>

            {ingestionInfo.metrics && ingestionInfo.metrics.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-2">
                {ingestionInfo.metrics.map((metric, idx) => (
                  <span key={idx} className={`text-xs font-medium px-2 py-1 rounded ${ingestionInfo.state === 'active'
                      ? 'bg-blue-100 text-blue-800'
                      : ingestionInfo.state === 'completed'
                        ? 'bg-green-100 text-green-800'
                        : 'bg-gray-100 text-gray-600'
                    }`}>
                    {metric}
                  </span>
                ))}
              </div>
            )}

            {ingestionInfo.percentage !== undefined && (
              <div className="mt-3">
                <div className="flex items-center justify-between mb-1">
                  <span className="text-xs font-medium text-blue-700">Progress</span>
                  <span className="text-xs font-semibold text-blue-900">{ingestionInfo.percentage}%</span>
                </div>
                <div className="w-full bg-blue-200 rounded-full h-2">
                  <div
                    className="bg-blue-600 h-2 rounded-full transition-all duration-300"
                    style={{ width: `${ingestionInfo.percentage}%` }}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Indexing Status Card */}
          <div className={`p-4 rounded-lg border-2 ${indexingInfo.state === 'active'
              ? 'border-indigo-200 bg-indigo-50'
              : indexingInfo.state === 'completed'
                ? 'border-green-200 bg-green-50'
                : 'border-gray-200 bg-gray-50'
            }`}>
            <div className="flex items-center justify-between mb-2">
              <div className="flex items-center gap-2">
                <div className={`w-2.5 h-2.5 rounded-full ${indexingInfo.state === 'active'
                    ? 'bg-indigo-500 animate-pulse'
                    : indexingInfo.state === 'completed'
                      ? 'bg-green-500'
                      : 'bg-gray-400'
                  }`}></div>
                <h3 className={`font-semibold ${indexingInfo.state === 'active'
                    ? 'text-indigo-900'
                    : indexingInfo.state === 'completed'
                      ? 'text-green-900'
                      : 'text-gray-700'
                  }`}>
                  {indexingInfo.title}
                </h3>
              </div>
              <span className={`text-xs font-medium px-2 py-1 rounded ${indexingInfo.state === 'active'
                  ? 'bg-indigo-100 text-indigo-700'
                  : indexingInfo.state === 'completed'
                    ? 'bg-green-100 text-green-700'
                    : 'bg-gray-200 text-gray-600'
                }`}>
                {indexingInfo.state === 'active' ? 'Active' : indexingInfo.state === 'completed' ? 'Complete' : 'Idle'}
              </span>
            </div>

            <p className={`text-sm ${indexingInfo.state === 'active'
                ? 'text-indigo-700'
                : indexingInfo.state === 'completed'
                  ? 'text-green-700'
                  : 'text-gray-500'
              }`}>
              {indexingInfo.detail}
            </p>

            {indexingInfo.metrics && indexingInfo.metrics.length > 0 && (
              <div className="mt-2 flex flex-wrap gap-2">
                {indexingInfo.metrics.map((metric, idx) => (
                  <span key={idx} className={`text-xs font-medium px-2 py-1 rounded ${indexingInfo.state === 'active'
                      ? 'bg-indigo-100 text-indigo-800'
                      : indexingInfo.state === 'completed'
                        ? 'bg-green-100 text-green-800'
                        : 'bg-gray-100 text-gray-600'
                    }`}>
                    {metric}
                  </span>
                ))}
              </div>
            )}
          </div>
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