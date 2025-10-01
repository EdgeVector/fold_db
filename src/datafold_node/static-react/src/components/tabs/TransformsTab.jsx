import { useState, useEffect, useMemo, useCallback } from 'react'
import { useAppSelector } from '../../store/hooks'
import { selectAllSchemas } from '../../store/schemaSlice'
import { transformClient } from '../../api/clients'

const INITIAL_QUEUE_STATE = {
  queue: [],
  length: 0,
  isEmpty: true
}

const getStateBadgeClasses = (state) => {
  const normalized = typeof state === 'string' ? state.toLowerCase() : ''

  switch (normalized) {
    case 'approved':
      return 'bg-green-100 text-green-800'
    case 'available':
      return 'bg-blue-100 text-blue-800'
    case 'blocked':
      return 'bg-red-100 text-red-800'
    default:
      return 'bg-gray-100 text-gray-700'
  }
}

const normalizeQueueInfo = (data = {}) => {
  const queue = Array.isArray(data.queue) ? data.queue : []
  const length = typeof data.length === 'number' ? data.length : queue.length
  const isEmpty = typeof data.isEmpty === 'boolean' ? data.isEmpty : queue.length === 0

  return { queue, length, isEmpty }
}

const getSchemaStateLabel = (state) => {
  if (typeof state !== 'string' || state.length === 0) {
    return 'Unknown'
  }

  return state.charAt(0).toUpperCase() + state.slice(1)
}

const TransformsTab = ({ onResult }) => {
  const schemas = useAppSelector(selectAllSchemas)
  const [queueInfo, setQueueInfo] = useState(INITIAL_QUEUE_STATE)
  const [loading, setLoading] = useState({})
  const [errors, setErrors] = useState({})
  const [isLoadingTransforms, setIsLoadingTransforms] = useState(false)
  const [transformsError, setTransformsError] = useState(null)
  const [apiTransforms, setApiTransforms] = useState([])

  const schemaTransforms = useMemo(() => {
    if (!schemas) {
      return []
    }

    return schemas.flatMap(schema => {
      if (!schema || typeof schema !== 'object') {
        return []
      }

      const fields = schema.fields && typeof schema.fields === 'object'
        ? Object.entries(schema.fields)
        : []

      return fields
        .filter(([, field]) => field && field.transform)
        .map(([fieldName, field]) => ({
          schemaName: schema.name,
          fieldName,
          transform: field.transform,
          schemaState: schema.state
        }))
    })
  }, [schemas])

  const fetchApiTransforms = useCallback(async () => {
    setIsLoadingTransforms(true)
    setTransformsError(null)

    try {
      const response = await transformClient.getTransforms()

      if (response?.success && response.data) {
        const data = response.data.data
        const normalized = Array.isArray(data)
          ? data
          : data && typeof data === 'object'
            ? Object.values(data)
            : []
        setApiTransforms(normalized)
      } else {
        const errorMessage = response?.error || 'Failed to load transforms'
        setTransformsError(errorMessage)
        setApiTransforms([])
      }
    } catch (error) {
      console.error('Failed to fetch API transforms:', error)
      setTransformsError(error.message || 'Failed to load transforms')
      setApiTransforms([])
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
    fetchApiTransforms()
    fetchQueueInfo()

    const interval = setInterval(fetchQueueInfo, 5000)
    return () => clearInterval(interval)
  }, [fetchApiTransforms, fetchQueueInfo])

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

      if (typeof transformClient.refreshQueue === 'function') {
        try {
          const refreshResponse = await transformClient.refreshQueue()
          if (refreshResponse?.success && refreshResponse.data) {
            setQueueInfo(normalizeQueueInfo(refreshResponse.data))
          }
        } catch (error) {
          console.error('Failed to refresh transform queue:', error)
        }
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
      <div className="flex justify-between items-center">
        <h2 className="text-xl font-semibold text-gray-800">Transforms</h2>
        <div className="text-sm text-gray-600">
          Queue Status: {queueInfo.isEmpty ? 'Empty' : `${queueInfo.length} transform(s) queued`}
        </div>
      </div>

      {!queueInfo.isEmpty && (
        <div className="bg-blue-50 p-4 rounded-lg" data-testid="transform-queue">
          <h3 className="text-md font-medium text-blue-800 mb-2">Transform Queue</h3>
          <ul className="list-disc list-inside space-y-1">
            {queueInfo.queue.map((transformId, index) => (
              <li key={`${transformId}-${index}`} className="text-blue-700">
                {transformId}
              </li>
            ))}
          </ul>
        </div>
      )}

      {isLoadingTransforms && (
        <div className="bg-blue-50 p-4 rounded-lg" role="status">
          <div className="flex items-center">
            <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-600 mr-2"></div>
            <span className="text-blue-800">Loading transforms...</span>
          </div>
        </div>
      )}

      {transformsError && (
        <div className="bg-red-50 p-4 rounded-lg" role="alert">
          <div className="flex items-center">
            <span className="text-red-800">Error loading transforms: {transformsError}</span>
            <button
              onClick={fetchApiTransforms}
              className="ml-4 px-3 py-1 text-sm bg-red-500 text-white rounded hover:bg-red-600"
            >
              Retry
            </button>
          </div>
        </div>
      )}

      {schemaTransforms.length > 0 && (
        <div className="space-y-4">
          {schemaTransforms.map(({ schemaName, fieldName, transform, schemaState }) => {
            const transformId = `${schemaName}.${fieldName}`
            const isLoading = loading[transformId]
            const errorMessage = errors[transformId]

            return (
              <div key={transformId} className="bg-white p-4 rounded-lg shadow">
                <div className="flex justify-between items-start">
                  <div>
                    <h3 className="text-lg font-medium text-gray-900">{schemaName}</h3>
                    <span className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium mt-1 ${getStateBadgeClasses(schemaState)}`}>
                      {getSchemaStateLabel(schemaState)}
                    </span>
                  </div>
                </div>

                <div className="mt-3 space-y-2">
                  <div className="text-sm text-gray-700">
                    <span className="font-medium">{fieldName}</span>
                  </div>

                  {transform?.logic && (
                    <div className="text-sm text-gray-600">
                      <span className="font-medium">Logic:</span> {transform.logic}
                    </div>
                  )}

                  {transform?.output && (
                    <div className="text-sm text-gray-600">
                      <span className="font-medium">Output:</span> {transform.output}
                    </div>
                  )}

                  {Array.isArray(transform?.inputs) && transform.inputs.length > 0 && (
                    <div className="text-sm text-gray-600">
                      <span className="font-medium">Inputs:</span> {transform.inputs.join(', ')}
                    </div>
                  )}
                </div>

                <div className="mt-4 flex items-center gap-3">
                  <button
                    onClick={() => handleAddToQueue(schemaName, fieldName)}
                    disabled={isLoading}
                    className={`px-3 py-1 text-sm rounded text-white ${
                      isLoading ? 'bg-blue-300 cursor-not-allowed' : 'bg-blue-500 hover:bg-blue-600'
                    }`}
                  >
                    {isLoading ? 'Adding...' : 'Add to Queue'}
                  </button>

                  {errorMessage && (
                    <span className="text-sm text-red-600">Error: {errorMessage}</span>
                  )}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {!transformsError && schemaTransforms.length === 0 && (
        <div className="bg-gray-50 p-4 rounded-lg">
          <p className="text-gray-600">No transforms found in schemas</p>
          <p className="text-sm text-gray-500 mt-1">
            Register a transform in a schema to view it here and add it to the processing queue.
          </p>
        </div>
      )}

      {!isLoadingTransforms && !transformsError && apiTransforms.length > 0 && (
        <div className="bg-green-50 p-4 rounded-lg">
          <h3 className="text-md font-medium text-green-800 mb-2">Registered API Transforms</h3>
          <ul className="space-y-1 text-sm text-green-700">
            {apiTransforms.map((transform, index) => {
              const identifier = typeof transform === 'string' ? transform : transform?.id || `transform-${index}`
              return (
                <li key={identifier}>
                  {typeof transform === 'string' ? transform : transform?.id || transform?.output}
                </li>
              )
            })}
          </ul>
        </div>
      )}
    </div>
  )
}

export default TransformsTab
