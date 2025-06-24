import { useState, useEffect } from 'react'

const TransformsTab = ({ schemas, _onResult }) => {
  const [transforms, setTransforms] = useState([])
  const [apiTransforms, setApiTransforms] = useState({})
  const [loading, setLoading] = useState({})
  const [error, setError] = useState({})
  const [debugInfo, setDebugInfo] = useState({})
  const [queueInfo, setQueueInfo] = useState({
    queue: [],
    length: 0,
    isEmpty: true
  })

  useEffect(() => {
    // Enhanced debug information
    const debug = {
      totalSchemas: schemas.length,
      schemaStates: {},
      transformFields: {},
      blockedSchemas: []
    }

    schemas.forEach(schema => {
      debug.schemaStates[schema.name] = schema.state
      debug.transformFields[schema.name] = []
      
      if (schema.fields) {
        Object.entries(schema.fields).forEach(([fieldName, field]) => {
          if (field.transform !== null && field.transform !== undefined) {
            debug.transformFields[schema.name].push({
              field: fieldName,
              transform: field.transform
            })
          }
        })
      }
      
      if (schema.state !== 'Approved') {
        debug.blockedSchemas.push({
          name: schema.name,
          state: schema.state
        })
      }
    })

    setDebugInfo(debug)

    // Filter and process schemas with transform fields - include ALL schemas regardless of state
    const transformSchemas = schemas.filter(schema => {
      // Include ALL schemas that have transforms, regardless of approval state
      const hasTransforms = schema.fields && Object.values(schema.fields).some(field =>
        field.transform !== null && field.transform !== undefined &&
        (field.transform || typeof field.transform === 'string')
      )
      return hasTransforms
    }).map(schema => {
      // Deep clone the schema to avoid modifying the original
      const processedSchema = JSON.parse(JSON.stringify(schema))

      // Process each field's transform
      Object.entries(processedSchema.fields).forEach(([fieldName, field]) => {
        if (typeof field.transform === 'string') {
          // Parse the transform string
          const match = field.transform.match(/transform\s+(\w+)\s*{\s*logic:\s*{\s*([^}]+);\s*}\s*}/)
          if (match) {
            field.transform = {
              logic: match[2].trim(),
              output: `${schema.name}.${fieldName}`,
              inputs: []
            }
          }
        } else if (field.transform && typeof field.transform === 'object') {
          // Ensure output field is set if missing
          if (!field.transform.output) {
            field.transform.output = `${schema.name}.${fieldName}`
          }
        }
      })
      return processedSchema
    })
    
    setTransforms(transformSchemas)

    // Fetch transforms from dedicated API
    const fetchApiTransforms = async () => {
      try {
        const response = await fetch('/api/transforms')
        const data = await response.json()
        setApiTransforms(data.data || {})
      } catch (error) {
        console.error('Failed to fetch API transforms:', error)
        setApiTransforms({})
      }
    }

    // Fetch queue information
    const fetchQueueInfo = async () => {
      try {
        const response = await fetch('/api/transforms/queue')
        if (response.ok) {
          const data = await response.json()
          setQueueInfo(data)
        } else {
          console.warn('Failed to fetch queue info, status:', response.status)
        }
      } catch (error) {
        console.error('Failed to fetch transform queue info:', error)
      }
    }

    fetchApiTransforms()
    fetchQueueInfo()
    // Poll for queue updates every 5 seconds
    const interval = setInterval(fetchQueueInfo, 5000)
    return () => clearInterval(interval)
  }, [schemas])

  const getStateColor = (state) => {
    switch (state?.toLowerCase()) {
      case 'approved':
        return 'bg-green-100 text-green-800'
      case 'available':
        return 'bg-blue-100 text-blue-800'
      case 'blocked':
        return 'bg-red-100 text-red-800'
      default:
        return 'bg-gray-100 text-gray-800'
    }
  }

  const handleAddToQueue = async (schemaName, fieldName, _transform) => {
    const transformId = `${schemaName}.${fieldName}`
    setLoading(prev => ({ ...prev, [transformId]: true }))
    setError(prev => ({ ...prev, [transformId]: null }))
    
    try {
      const response = await fetch(`/api/transforms/queue/${transformId}`, {
        method: 'POST'
      })
      const responseData = await response.json()
      
      if (!response.ok) {
        throw new Error(responseData.error || 'Failed to add transform to queue')
      }
      
      // Refresh queue info immediately
      const queueResponse = await fetch('/api/transforms/queue')
      const data = await queueResponse.json()
      setQueueInfo(data)
    } catch (error) {
      console.error('Failed to add transform to queue:', error)
      setError(prev => ({ ...prev, [transformId]: error.message }))
    } finally {
      setLoading(prev => ({ ...prev, [transformId]: false }))
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <h2 className="text-xl font-semibold text-gray-800">Transforms</h2>
        <div className="text-sm text-gray-600">
          Queue Status: {queueInfo.isEmpty ? 'Empty' : `${queueInfo.length} transform(s) queued`}
        </div>
      </div>

      {!queueInfo.isEmpty && (
        <div className="bg-blue-50 p-4 rounded-lg mb-4">
          <h3 className="text-md font-medium text-blue-800 mb-2">Transform Queue</h3>
          <ul className="list-disc list-inside space-y-1">
            {(queueInfo.queue || []).map((transformId, index) => (
              <li key={index} className="text-blue-700">
                {transformId}
              </li>
            ))}
          </ul>
        </div>
      )}

      {transforms.length === 0 ? (
        <p className="text-gray-500">No transforms found in schemas</p>
      ) : (
        <div className="space-y-6">
          {transforms.map((schema) => (
            <div key={schema.name} className="bg-white shadow rounded-lg p-4">
              <div className="flex items-center space-x-3 mb-2">
                <h3 className="text-lg font-medium text-gray-800">{schema.name}</h3>
                <span className={`px-2 py-1 text-xs font-medium rounded-full ${getStateColor(schema.state)}`}>
                  {schema.state || 'Unknown'}
                </span>
              </div>
              <div className="space-y-4">
                {Object.entries(schema.fields).map(([fieldName, field]) => {
                  if (!field.transform) return null
                  return (
                    <div key={fieldName} className="border-l-4 border-primary pl-4">
                      <h4 className="font-medium text-gray-700">{fieldName}</h4>
                      <div className="mt-2 space-y-2">
                      <div className="text-sm">
                          <div className="flex items-center gap-2">
                            <span className="font-medium">Output:</span>
                            <span className="text-blue-600">{field.transform.output}</span>
                          </div>
                          <div className="flex items-center">
                            <button
                              onClick={() => handleAddToQueue(schema.name, fieldName, field.transform)}
                              disabled={loading[`${schema.name}.${fieldName}`]}
                              className="ml-4 px-3 py-1 text-sm bg-blue-500 text-white rounded hover:bg-blue-600 disabled:bg-blue-300"
                            >
                              {loading[`${schema.name}.${fieldName}`] ? 'Adding...' : 'Add to Queue'}
                            </button>
                            {error[`${schema.name}.${fieldName}`] && (
                              <span className="ml-2 text-sm text-red-600">
                                Error: {error[`${schema.name}.${fieldName}`]}
                              </span>
                            )}
                          </div>
                        </div>
                        <div className="text-sm">
                          <span className="font-medium">Logic:</span>{' '}
                          <code className="bg-gray-100 px-2 py-1 rounded text-gray-800">
                            {field.transform.logic}
                          </code>
                        </div>
                        {field.transform.output && (
                          <div className="text-sm mt-2 bg-blue-50 p-3 rounded-md border-l-4 border-blue-500">
                            <span className="font-medium text-blue-700">Output:</span>{' '}
                            <code className="ml-1">{field.transform.output}</code>
                          </div>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default TransformsTab