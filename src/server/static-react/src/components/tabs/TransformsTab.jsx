import { useState, useEffect, useCallback } from 'react'
import { transformClient } from '../../api/clients'

const INITIAL_QUEUE_STATE = {
  queue: [],
  length: 0,
  isEmpty: true
}

const normalizeQueueInfo = (data = {}) => {
  const queue = Array.isArray(data.queue) ? data.queue : []
  const length = typeof data.length === 'number' ? data.length : queue.length
  const isEmpty = typeof data.isEmpty === 'boolean' ? data.isEmpty : queue.length === 0

  return { queue, length, isEmpty }
}

const TransformsTab = ({ onResult }) => {
  const [queueInfo, setQueueInfo] = useState(INITIAL_QUEUE_STATE)
  const [loading, setLoading] = useState({})
  const [errors, setErrors] = useState({})
  const [isLoadingTransforms, setIsLoadingTransforms] = useState(false)
  const [transformsError, setTransformsError] = useState(null)
  const [transforms, setTransforms] = useState([])

  const fetchTransforms = useCallback(async () => {
    setIsLoadingTransforms(true)
    setTransformsError(null)

    try {
      const response = await transformClient.getTransforms()

      if (response?.success && response.data) {
        const data = response.data
        // Backend returns HashMap<String, Transform> where Transform has flattened schema fields
        // Convert to array with transform_id extracted from the key
        const normalized = data && typeof data === 'object' && !Array.isArray(data)
          ? Object.entries(data).map(([transformId, transform]) => ({
              transform_id: transformId,
              ...transform
            }))
          : Array.isArray(data)
            ? data
            : []
        setTransforms(normalized)
      } else {
        const errorMessage = response?.error || 'Failed to load transforms'
        setTransformsError(errorMessage)
        setTransforms([])
      }
    } catch (error) {
      console.error('Failed to fetch transforms:', error)
      setTransformsError(error.message || 'Failed to load transforms')
      setTransforms([])
    } finally {
      setIsLoadingTransforms(false)
    }
  }, [])

  const fetchQueueInfo = useCallback(async () => {
    try {
      const response = await transformClient.getQueue()

      if (response?.success && response.data) {
        setQueueInfo(normalizeQueueInfo(response.data))
      }
    } catch (error) {
      console.error('Failed to fetch transform queue info:', error)
    }
  }, [])

  // Fetch transforms and queue info on mount
  useEffect(() => {
    fetchTransforms()
    fetchQueueInfo()

    const interval = setInterval(fetchQueueInfo, 5000)
    return () => clearInterval(interval)
  }, [fetchTransforms, fetchQueueInfo])

  const handleAddToQueue = useCallback(async (schemaName, fieldName) => {
    const transformId = fieldName ? `${schemaName}.${fieldName}` : schemaName

    setLoading(prev => ({ ...prev, [transformId]: true }))
    setErrors(prev => ({ ...prev, [transformId]: null }))

    try {
      const response = await transformClient.addToQueue(transformId)

      if (!response?.success) {
        const message = response?.data?.message || response?.error || 'Failed to add transform to queue'
        throw new Error(message)
      }

      if (typeof onResult === 'function') {
        onResult({ success: true, transformId })
      }

      await fetchQueueInfo()
    } catch (error) {
      console.error('Failed to add transform to queue:', error)
      setErrors(prev => ({ ...prev, [transformId]: error.message || 'Failed to add transform to queue' }))
    } finally {
      setLoading(prev => ({ ...prev, [transformId]: false }))
    }
  }, [fetchQueueInfo, onResult])

  return (
    <div className="p-6 space-y-4">
      <div className="flex justify-end">
        <span className="text-sm text-secondary">
          Queue: {queueInfo.isEmpty ? 'Empty' : `${queueInfo.length} queued`}
        </span>
      </div>

      {!queueInfo.isEmpty && (
        <div className="card card-info p-4" data-testid="transform-queue">
          <p className="text-sm font-medium text-info mb-2">Queue</p>
          <ul className="list-none space-y-1">
            {queueInfo.queue.map((transformId, index) => (
              <li key={`${transformId}-${index}`} className="text-secondary font-mono text-sm">
                → {transformId}
              </li>
            ))}
          </ul>
        </div>
      )}

      {isLoadingTransforms && (
        <div className="flex items-center gap-2" role="status">
          <span className="spinner" />
          <span className="text-secondary">Loading transforms...</span>
        </div>
      )}

      {transformsError && (
        <div className="card card-error p-4" role="alert">
          <div className="flex items-center gap-4">
            <span className="text-error">Error loading transforms: {transformsError}</span>
            <button onClick={fetchTransforms} className="btn-secondary btn-sm">Retry</button>
          </div>
        </div>
      )}

      {!isLoadingTransforms && !transformsError && transforms.length > 0 && (
        <div className="space-y-4">
          {transforms.map((transform, index) => {
            // Transform has flattened schema fields due to #[serde(flatten)] in Rust
            const transformId = transform.transform_id || `transform-${index}`
            const isLoading = loading[transformId]
            const errorMessage = errors[transformId]

            // Extract schema name from transform_id or use the name field
            const schemaName = transform.name || transform.transform_id?.split('.')[0] || 'Unknown'

            // Determine schema type - fields are flattened, so access directly
            const schemaType = transform.schema_type
            let schemaTypeLabel = 'Single'
            let schemaTypeColor = 'badge badge-neutral'

            if (schemaType?.Range) {
              schemaTypeLabel = 'Range'
              schemaTypeColor = 'badge badge-info'
            } else if (schemaType?.HashRange) {
              schemaTypeLabel = 'HashRange'
              schemaTypeColor = 'badge badge-neutral'
            }

            // Get key configuration and transform fields - flattened
            const keyConfig = transform.key
            const transformFieldsObj = transform.transform_fields || {}
            const transformFieldsCount = Object.keys(transformFieldsObj).length
            const fieldNames = Object.keys(transformFieldsObj)

            return (
              <div key={transformId} className="card p-4">
                <div className="flex justify-between items-start mb-3">
                  <div className="flex-1">
                    <p className="text-lg font-semibold text-primary">{schemaName}</p>
                    <div className="flex gap-2 mt-2 flex-wrap">
                      <span className={schemaTypeColor}>{schemaTypeLabel}</span>
                      {transformFieldsCount > 0 && (
                        <span className="badge badge-success">
                          {transformFieldsCount} field{transformFieldsCount !== 1 ? 's' : ''}
                        </span>
                      )}
                    </div>
                    {fieldNames.length > 0 && (
                      <p className="mt-2 text-sm text-secondary">
                        <span className="font-medium">Fields:</span> {fieldNames.join(', ')}
                      </p>
                    )}
                  </div>
                </div>

                <div className="mt-3 space-y-3">
                  {keyConfig && (
                    <div className="bg-surface-secondary p-3 border-l-2 border-info">
                      <p className="text-sm font-medium text-info mb-1">--key-config:</p>
                      <div className="text-sm text-secondary space-y-1 font-mono">
                        {keyConfig.hash_field && <p>hash_key: {keyConfig.hash_field}</p>}
                        {keyConfig.range_field && <p>range_key: {keyConfig.range_field}</p>}
                        {!keyConfig.hash_field && !keyConfig.range_field && keyConfig.key_field && (
                          <p>key: {keyConfig.key_field}</p>
                        )}
                      </div>
                    </div>
                  )}

                  {transformFieldsCount > 0 && (
                    <div>
                      <p className="text-sm font-medium text-secondary mb-2">--transform-fields:</p>
                      <div className="bg-surface-secondary p-3 space-y-2">
                        {Object.entries(transformFieldsObj).map(([fieldName, logic]) => (
                          <div key={fieldName} className="border-l-2 border-border pl-3">
                            <p className="font-medium text-primary text-sm">{fieldName}</p>
                            <p className="text-secondary font-mono text-xs mt-1 break-all">{logic}</p>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>

                <div className="mt-4 flex items-center gap-3">
                  <button
                    onClick={() => handleAddToQueue(transformId, null)}
                    disabled={isLoading}
                    className="btn-primary"
                  >
                    {isLoading ? '→ Adding...' : '→ Add to Queue'}
                  </button>
                  {errorMessage && <span className="text-sm text-error">Error: {errorMessage}</span>}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {!isLoadingTransforms && !transformsError && transforms.length === 0 && (
        <p className="text-secondary">No transforms registered. Register a transform in a schema to view it here.</p>
      )}
    </div>
  )
}

export default TransformsTab
