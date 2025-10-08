import { useState, useEffect, useCallback } from 'react'
import { transformClient } from '../api/clients'

// Status configuration - no branching logic
const STATUS_CONFIG = {
  InProgress: { color: 'text-blue-700 bg-blue-50', icon: '⏳' },
  Completed: { color: 'text-green-700 bg-green-50', icon: '✅' },
  Failed: { color: 'text-red-700 bg-red-50', icon: '❌' },
  default: { color: 'text-gray-700 bg-gray-50', icon: '❓' }
}

const formatTime = (timestamp) => {
  const date = new Date(timestamp * 1000)
  return date.toLocaleString()
}

const formatDuration = (startTime, endTime) => {
  const duration = (endTime || Math.floor(Date.now() / 1000)) - startTime
  if (duration < 60) return `${duration}s`
  if (duration < 3600) return `${Math.floor(duration / 60)}m ${duration % 60}s`
  return `${Math.floor(duration / 3600)}h ${Math.floor((duration % 3600) / 60)}m`
}

const calculateSuccessRate = (completed, failed) => {
  const total = completed + failed
  if (total === 0) return 'N/A'
  return `${Math.round((completed / total) * 100)}%`
}

const BackfillCard = ({ backfill }) => {
  const statusConfig = STATUS_CONFIG[backfill.status] || STATUS_CONFIG.default
  
  return (
    <div className={`p-3 rounded-lg border ${statusConfig.color}`}>
      <div className="flex justify-between items-start mb-2">
        <div className="flex items-center gap-2">
          <span className="text-xl">{statusConfig.icon}</span>
          <div>
            <div className="font-semibold">{backfill.transform_id}</div>
            <div className="text-xs opacity-80">Source: {backfill.source_schema}</div>
          </div>
        </div>
        <div className="text-xs text-right">
          <div>Started: {formatTime(backfill.start_time)}</div>
          <div>Duration: {formatDuration(backfill.start_time, backfill.end_time)}</div>
        </div>
      </div>

      <BackfillDetails backfill={backfill} />
      {backfill.status === 'InProgress' && backfill.mutations_expected > 0 && (
        <BackfillProgress backfill={backfill} />
      )}
    </div>
  )
}

const BackfillDetails = ({ backfill }) => {
  const { status } = backfill

  if (status === 'InProgress') {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2">
        <div>
          <span className="font-medium">Mutations:</span> {backfill.mutations_completed} / {backfill.mutations_expected}
        </div>
        {backfill.mutations_failed > 0 && (
          <div className="text-red-600">
            <span className="font-medium">Failed:</span> {backfill.mutations_failed}
          </div>
        )}
      </div>
    )
  }

  if (status === 'Completed') {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2">
        <div>
          <span className="font-medium">Mutations:</span> {backfill.mutations_completed}
        </div>
        <div>
          <span className="font-medium">Records:</span> {backfill.records_produced}
        </div>
        <div>
          <span className="font-medium">Completed:</span> {backfill.end_time && formatTime(backfill.end_time)}
        </div>
      </div>
    )
  }

  if (status === 'Failed' && backfill.error) {
    return (
      <div className="grid grid-cols-2 md:grid-cols-3 gap-2 text-sm mt-2">
        <div className="col-span-2 md:col-span-3">
          <span className="font-medium">Error:</span> {backfill.error}
        </div>
      </div>
    )
  }

  return null
}

const BackfillProgress = ({ backfill }) => {
  const percentage = Math.round((backfill.mutations_completed / backfill.mutations_expected) * 100)
  
  return (
    <div className="mt-2">
      <div className="w-full bg-gray-200 rounded-full h-2">
        <div
          className="bg-blue-600 h-2 rounded-full transition-all duration-300"
          style={{ width: `${percentage}%` }}
        ></div>
      </div>
      <div className="text-xs text-right mt-1">
        {percentage}% complete
      </div>
    </div>
  )
}

const BackfillMonitor = () => {
  const [backfills, setBackfills] = useState([])
  const [statistics, setStatistics] = useState(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState(null)
  const [showAll, setShowAll] = useState(false)

  const fetchBackfills = useCallback(async () => {
    try {
      const response = await transformClient.getAllBackfills()
      if (!response?.success || !response.data) {
        throw new Error(response?.error || 'Failed to fetch backfills - invalid response')
      }
      setBackfills(response.data)
      setError(null)
    } catch (err) {
      console.error('Failed to fetch backfills:', err)
      setError(err.message || 'Failed to load backfills')
      throw err
    }
  }, [])

  const fetchStatistics = useCallback(async () => {
    try {
      const response = await transformClient.getBackfillStatistics()
      if (!response?.success || !response.data) {
        throw new Error(response?.error || 'Failed to fetch backfill statistics - invalid response')
      }
      setStatistics(response.data)
      setError(null)
    } catch (err) {
      console.error('Failed to fetch backfill statistics:', err)
      setError(err.message || 'Failed to load statistics')
      throw err
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchBackfills()
    fetchStatistics()

    const interval = setInterval(() => {
      fetchBackfills()
      fetchStatistics()
    }, 3000) // Poll every 3 seconds

    return () => clearInterval(interval)
  }, [fetchBackfills, fetchStatistics])

  const activeBackfills = backfills.filter(b => b.status === 'InProgress')
  const completedBackfills = backfills.filter(b => b.status === 'Completed')
  const failedBackfills = backfills.filter(b => b.status === 'Failed')
  const displayedBackfills = showAll ? backfills : activeBackfills

  if (loading) {
    return (
      <div className="bg-gray-50 p-4 rounded-lg">
        <div className="flex items-center">
          <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-600 mr-2"></div>
          <span className="text-gray-800">Loading backfill information...</span>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="bg-red-50 p-4 rounded-lg" role="alert">
        <span className="text-red-800">Error: {error}</span>
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* Backfill Statistics Summary */}
      {statistics && (
        <div className="bg-gray-50 p-4 rounded-lg">
          <h3 className="text-md font-medium text-gray-800 mb-3">Backfill Statistics</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <div className="text-gray-600">Total Mutations</div>
              <div className="text-lg font-semibold text-gray-900">{statistics.total_mutations_completed}</div>
            </div>
            <div>
              <div className="text-gray-600">Success Rate</div>
              <div className="text-lg font-semibold text-green-700">
                {calculateSuccessRate(statistics.total_mutations_completed, statistics.total_mutations_failed)}
              </div>
            </div>
            <div>
              <div className="text-gray-600">Backfills</div>
              <div className="text-lg font-semibold text-blue-700">{statistics.total_backfills}</div>
            </div>
            <div>
              <div className="text-gray-600">Failures</div>
              <div className="text-lg font-semibold text-red-700">{statistics.total_mutations_failed}</div>
            </div>
          </div>
        </div>
      )}

      {/* Backfills Section */}
      <div className="bg-gray-50 p-4 rounded-lg">
        <div className="flex justify-between items-center mb-3">
          <h3 className="text-md font-medium text-gray-800">Backfills</h3>
          <div className="flex items-center gap-4">
            <div className="text-sm text-gray-600">
              Active: {activeBackfills.length} | Completed: {completedBackfills.length} | Failed: {failedBackfills.length}
            </div>
            <button
              onClick={() => setShowAll(!showAll)}
              className="px-3 py-1 text-sm bg-gray-200 text-gray-800 rounded hover:bg-gray-300"
            >
              {showAll ? 'Show Active Only' : 'Show All'}
            </button>
          </div>
        </div>

        {displayedBackfills.length === 0 ? (
          <div className="text-gray-600 text-sm">
            {showAll ? 'No backfills recorded' : 'No active backfills'}
          </div>
        ) : (
          <div className="space-y-3">
            {displayedBackfills.map((backfill) => (
              <BackfillCard 
                key={`${backfill.transform_id}-${backfill.start_time}`} 
                backfill={backfill} 
              />
            ))}
          </div>
        )}
      </div>
    </div>
  )
}

export default BackfillMonitor
