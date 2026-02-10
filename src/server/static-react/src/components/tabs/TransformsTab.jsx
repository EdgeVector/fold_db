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
    <div className="space-y-4">
      <div className="flex justify-end">
        <div className="text-sm text-secondary">
          Queue: {queueInfo.isEmpty ? 'Empty' : `${queueInfo.length} queued`}
        </div>
      </div>

      {!queueInfo.isEmpty && (
        <div className="border border-border bg-surface p-4 border-l-4 border-l-blue-300" data-testid="transform-queue">
          <h3 className="text-md font-medium text-info mb-2">Queue</h3>
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
        <div className="border border-border bg-surface p-4" role="status">
          <div className="flex items-center">
            <div className="animate-spin h-4 w-4 border-b-2 border-green-500 mr-2"></div>
            <span className="text-secondary">Loading transforms...</span>
          </div>
        </div>
      )}

      {transformsError && (
        <div className="border border-border bg-surface p-4 border-l-4 border-l-red-500" role="alert">
          <div className="flex items-center">
            <span className="text-error">Error loading transforms: {transformsError}</span>
            <button
              onClick={fetchTransforms}
              className="ml-4 px-3 py-1.5 text-sm text-secondary bg-transparent border border-border cursor-pointer hover:border-primary hover:text-primary transition-colors"
            >
              Retry
            </button>
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
              <div key={transformId} className="border border-border bg-surface p-4 border-l-4 border-l-border">
                <div className="flex justify-between items-start mb-3">
                  <div className="flex-1">
                    <h3 className="text-lg font-semibold text-primary">{schemaName}</h3>
                    <div className="flex gap-2 mt-2 flex-wrap">
                      <span className={`inline-flex items-center px-2.5 py-0.5 text-xs font-medium ${schemaTypeColor}`}>
                        {schemaTypeLabel}
                      </span>
                      {transformFieldsCount > 0 && (
                        <span className="badge badge-success">
                          {transformFieldsCount} field{transformFieldsCount !== 1 ? 's' : ''}
                        </span>
                      )}
                    </div>
                    {fieldNames.length > 0 && (
                      <div className="mt-2 text-sm text-secondary">
                        <span className="font-medium">Fields:</span> {fieldNames.join(', ')}
                      </div>
                    )}
                  </div>
                </div>

                <div className="mt-3 space-y-3">
                  {/* Key Configuration */}
                  {keyConfig && (
                    <div className="border border-border bg-surface p-3 border-l-2 border-l-blue-300">
                      <div className="text-sm font-medium text-info mb-1">--key-config:</div>
                      <div className="text-sm text-secondary space-y-1 font-mono">
                        {keyConfig.hash_field && (
                          <div>
                            hash_key: {keyConfig.hash_field}
                          </div>
                        )}
                        {keyConfig.range_field && (
                          <div>
                            range_key: {keyConfig.range_field}
                          </div>
                        )}
                        {!keyConfig.hash_field && !keyConfig.range_field && keyConfig.key_field && (
                          <div>
                            key: {keyConfig.key_field}
                          </div>
                        )}
                      </div>
                    </div>
                  )}
                  
                  {/* Transform Fields */}
                  {transformFieldsCount > 0 && (
                    <div>
                      <div className="text-sm font-medium text-secondary mb-2">--transform-fields:</div>
                      <div className="border border-border bg-surface p-3 space-y-2">
                        {Object.entries(transformFieldsObj).map(([fieldName, logic]) => (
                          <div key={fieldName} className="border-l-2 border-default pl-3">
                            <div className="font-medium text-primary text-sm">{fieldName}</div>
                            <div className="text-secondary font-mono text-xs mt-1 break-all">{logic}</div>
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
                    className="px-4 py-2 text-sm bg-primary text-white border-none cursor-pointer hover:bg-accent-hover transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    {isLoading ? '→ Adding...' : '→ Add to Queue'}
                  </button>

                  {errorMessage && (
                    <span className="text-sm text-error">Error: {errorMessage}</span>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {!isLoadingTransforms && !transformsError && transforms.length === 0 && (
        <div className="border border-border bg-surface p-4">
          <p className="text-secondary">No transforms registered</p>
          <p className="text-sm text-secondary mt-1">
            Register a transform in a schema to view it here and add it to the processing queue.
          </p>
        </div>
      )}
    </div>
  )
}

export default TransformsTab
