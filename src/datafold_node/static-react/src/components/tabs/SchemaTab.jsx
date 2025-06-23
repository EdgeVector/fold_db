import { useState, useEffect } from 'react'
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/solid'
import { getRangeSchemaInfo } from '../../utils/rangeSchemaUtils'
import { API_ENDPOINTS } from '../../api/endpoints'

function SchemaTab({ onResult, onSchemaUpdated }) {
  const [expandedSchemas, setExpandedSchemas] = useState({})
  const [, setSampleSchemas] = useState([])
  const [selectedSample, setSelectedSample] = useState('')
  const [, setLoadingSample] = useState(false)
  const [, setSamplesError] = useState(null)
  const [allSchemas, setAllSchemas] = useState([])

  useEffect(() => {
    fetchSampleSchemas()
    fetchAllSchemas()
  }, [])

  const fetchSampleSchemas = async () => {
    try {
      const resp = await fetch('/api/samples/schemas')
      const data = await resp.json()
      setSampleSchemas(data.data || [])
    } catch (err) {
      console.error('Failed to fetch sample schemas:', err)
      setSamplesError('Failed to load sample schemas')
    }
  }

  const fetchAllSchemas = async () => {
    try {
      const resp = await fetch('/api/schemas')
      const data = await resp.json()
      // Convert the state map to an array of schema objects with states
      const schemasWithStates = Object.entries(data.data || {}).map(([name, state]) => ({
        name,
        state,
        fields: {} // Will be populated when expanded
      }))
      setAllSchemas(schemasWithStates)
    } catch (err) {
      console.error('Failed to fetch all schemas:', err)
    }
  }

  const _loadSchema = async (schemaName) => {
    try {
      const resp = await fetch(`/api/schema/${schemaName}/load`, { method: 'POST' })
      const data = await resp.json()
      
      if (!resp.ok) {
        throw new Error(data.error || `Failed to load schema: ${resp.status}`)
      }
      
      if (onResult) {
        onResult(data)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
      // Refresh the schema list
      fetchAllSchemas()
    } catch (err) {
      console.error('Failed to load schema:', err)
      if (onResult) {
        onResult({ error: `Failed to load schema: ${err.message}` })
      }
    }
  }

  const toggleSchema = async (schemaName) => {
    const isCurrentlyExpanded = expandedSchemas[schemaName]
    
    setExpandedSchemas(prev => ({
      ...prev,
      [schemaName]: !prev[schemaName]
    }))

    // If expanding and schema doesn't have fields yet, fetch them
    if (!isCurrentlyExpanded) {
      const schema = allSchemas.find(s => s.name === schemaName)
      if (schema && (!schema.fields || Object.keys(schema.fields).length === 0)) {
        try {
          const resp = await fetch(`/api/schema/${schemaName}`)
          console.log('resp', resp)
          if (resp.ok) {
            const schemaData = await resp.json()
            // Update the schema with field details
            setAllSchemas(prev => prev.map(s =>
              s.name === schemaName
                ? { ...s, fields: schemaData.fields || {}, fieldsLoaded: true }
                : s
            ))
          }
        } catch (err) {
          console.error(`Failed to fetch schema details for ${schemaName}:`, err)
        }
      }
    }
  }

  const _removeSchema = async (schemaName) => {
    try {
      const resp = await fetch(`/api/schema/${schemaName}`, { method: 'DELETE' })
      if (!resp.ok) {
        throw new Error(`Failed to unload schema: ${resp.status}`)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
    } catch (err) {
      console.error('Failed to unload schema:', err)
    }
  }

  const _loadSampleSchema = async () => {
    if (!selectedSample) return
    setLoadingSample(true)
    setSamplesError(null)

    try {
      const resp = await fetch(`/api/samples/schema/${selectedSample}`)
      if (!resp.ok) {
        throw new Error(`Failed to fetch sample: ${resp.status}`)
      }
      const schema = await resp.json()
      const createResp = await fetch(API_ENDPOINTS.SCHEMA, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(schema)
      })

      const data = await createResp.json()

      if (!createResp.ok) {
        throw new Error(data.error || 'Failed to load schema')
      }

      if (onResult) {
        onResult(data)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
      setSelectedSample('')
    } catch (err) {
      console.error('Failed to load sample schema:', err)
      setSamplesError('Failed to load sample schema')
      if (onResult) {
        onResult({ error: 'Failed to load sample schema' })
      }
    } finally {
      setLoadingSample(false)
    }
  }

  const renderField = (field, fieldName, isRangeKey = false) => {
    const formatPermissionPolicy = (policy) => {
      if (!policy) return 'Unknown'
      if (policy.NoRequirement !== undefined) return 'No Requirement'
      if (policy.Distance !== undefined) return `Trust Distance ${policy.Distance}`
      return 'Unknown'
    }

    return (
      <div key={fieldName} className={`rounded-md p-4 hover:bg-gray-100 transition-colors duration-200 ${
        isRangeKey ? 'bg-purple-50 border border-purple-200' : 'bg-gray-50'
      }`}>
        <div className="flex justify-between items-start">
          <div className="space-y-2">
            <div className="flex items-center">
              <span className="font-medium text-gray-900">{fieldName}</span>
              <span className="ml-2 px-2 py-0.5 text-xs font-medium rounded-full bg-gray-200 text-gray-700">
                {field.field_type}
              </span>
              {isRangeKey && (
                <span className="ml-2 px-2 py-0.5 text-xs font-medium rounded-full bg-purple-200 text-purple-800">
                  Range Key
                </span>
              )}
            </div>
            
            {/* Permission Policies */}
            {field.permission_policy && (
              <div className="space-y-1">
                <div className="flex items-center text-xs text-gray-600">
                  <span className="font-medium mr-2">Read:</span>
                  <span className="px-1.5 py-0.5 bg-blue-100 text-blue-800 rounded">
                    {formatPermissionPolicy(field.permission_policy.read_policy)}
                  </span>
                </div>
                <div className="flex items-center text-xs text-gray-600">
                  <span className="font-medium mr-2">Write:</span>
                  <span className="px-1.5 py-0.5 bg-orange-100 text-orange-800 rounded">
                    {formatPermissionPolicy(field.permission_policy.write_policy)}
                  </span>
                </div>
              </div>
            )}
            
            {field.transform && (
              <div className="flex items-center text-sm text-gray-600">
                <svg className="icon icon-xs mr-1" viewBox="0 0 20 20" fill="currentColor">
                  <path fillRule="evenodd" d="M11.3 1.046A1 1 0 0112 2v5h4a1 1 0 01.82 1.573l-7 10A1 1 0 018 18v-5H4a1 1 0 01-.82-1.573l7-10a1 1 0 011.12-.38z" clipRule="evenodd" />
                </svg>
                {field.transform.name}
              </div>
            )}
            {field.ref_atom_uuid && (
              <div className="text-xs text-gray-500 break-all">
                {field.ref_atom_uuid}
              </div>
            )}
          </div>
          <span className={`
            inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium
            ${field.writable
              ? 'bg-green-100 text-green-800'
              : 'bg-gray-100 text-gray-800'
            }
          `}>
            {field.writable ? 'Writable' : 'Read-only'}
          </span>
        </div>
      </div>
    )
  }

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

  const approveSchema = async (schemaName) => {
    try {
      const resp = await fetch(`/api/schema/${schemaName}/approve`, { method: 'POST' })
      const data = await resp.json()
      
      if (!resp.ok) {
        throw new Error(data.error || `Failed to approve schema: ${resp.status}`)
      }
      
      if (onResult) {
        onResult(data)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
      // Refresh the schema list
      fetchAllSchemas()
    } catch (err) {
      console.error('Failed to approve schema:', err)
      if (onResult) {
        onResult({ error: `Failed to approve schema: ${err.message}` })
      }
    }
  }

  const blockSchema = async (schemaName) => {
    try {
      const resp = await fetch(`/api/schema/${schemaName}/block`, { method: 'POST' })
      const data = await resp.json()
      
      if (!resp.ok) {
        throw new Error(data.error || `Failed to block schema: ${resp.status}`)
      }
      
      if (onResult) {
        onResult(data)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
      // Refresh the schema list
      fetchAllSchemas()
    } catch (err) {
      console.error('Failed to block schema:', err)
      if (onResult) {
        onResult({ error: `Failed to block schema: ${err.message}` })
      }
    }
  }

  const unloadSchema = async (schemaName) => {
    try {
      const resp = await fetch(`/api/schema/${schemaName}`, { method: 'DELETE' })
      const data = await resp.json()
      
      if (!resp.ok) {
        throw new Error(data.error || `Failed to unload schema: ${resp.status}`)
      }
      
      if (onResult) {
        onResult(data)
      }
      if (onSchemaUpdated) {
        onSchemaUpdated()
      }
      // Refresh the schema list
      fetchAllSchemas()
    } catch (err) {
      console.error('Failed to unload schema:', err)
      if (onResult) {
        onResult({ error: `Failed to unload schema: ${err.message}` })
      }
    }
  }

  const renderSchema = (schema) => {
    const isExpanded = expandedSchemas[schema.name]
    const state = schema.state || 'Unknown'
    const rangeSchemaInfo = schema.fields ? getRangeSchemaInfo(schema) : null

    return (
      <div key={schema.name} className="bg-white rounded-lg border border-gray-200 shadow-sm overflow-hidden transition-all duration-200 hover:shadow-md">
        <div
          className="px-4 py-3 bg-gray-50 cursor-pointer select-none transition-colors duration-200 hover:bg-gray-100"
          onClick={() => toggleSchema(schema.name)}
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              {isExpanded ? (
                <ChevronDownIcon className="icon icon-sm text-gray-400 transition-transform duration-200" />
              ) : (
                <ChevronRightIcon className="icon icon-sm text-gray-400 transition-transform duration-200" />
              )}
              <h3 className="font-medium text-gray-900">{schema.name}</h3>
              <span className={`px-2 py-1 text-xs font-medium rounded-full ${getStateColor(state)}`}>
                {state}
              </span>
              {rangeSchemaInfo && (
                <span className="px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
                  Range Schema
                </span>
              )}
            </div>
            <div className="flex items-center space-x-2">
              {state.toLowerCase() === 'available' && (
                <button
                  className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
                  onClick={(e) => {
                    e.stopPropagation()
                    approveSchema(schema.name)
                  }}
                >
                  Approve
                </button>
              )}
              {state.toLowerCase() === 'approved' && (
                <>
                  <button
                    className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-orange-600 hover:bg-orange-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-orange-500"
                    onClick={(e) => {
                      e.stopPropagation()
                      unloadSchema(schema.name)
                    }}
                  >
                    Unload
                  </button>
                  <button
                    className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
                    onClick={(e) => {
                      e.stopPropagation()
                      blockSchema(schema.name)
                    }}
                  >
                    Block
                  </button>
                </>
              )}
              {state.toLowerCase() === 'blocked' && (
                <button
                  className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
                  onClick={(e) => {
                    e.stopPropagation()
                    approveSchema(schema.name)
                  }}
                >
                  Re-approve
                </button>
              )}
            </div>
          </div>
        </div>
        
        {isExpanded && schema.fields && (
          <div className="p-4 border-t border-gray-200">
            {/* Range Schema Information */}
            {rangeSchemaInfo && (
              <div className="mb-4 p-3 bg-purple-50 rounded-md border border-purple-200">
                <h4 className="text-sm font-medium text-purple-900 mb-2">Range Schema Information</h4>
                <div className="space-y-1 text-xs text-purple-800">
                  <p><strong>Range Key:</strong> {rangeSchemaInfo.rangeKey}</p>
                  <p><strong>Total Fields:</strong> {rangeSchemaInfo.totalFields}</p>
                  <p><strong>Range Fields:</strong> {rangeSchemaInfo.rangeFields.length}</p>
                  <p className="text-purple-600">
                    This schema uses range-based storage for efficient querying and mutations.
                  </p>
                </div>
              </div>
            )}
            
            <div className="space-y-3">
              {Object.entries(schema.fields).map(([fieldName, field]) =>
                renderField(field, fieldName, rangeSchemaInfo?.rangeKey === fieldName)
              )}
            </div>
          </div>
        )}
      </div>
    )
  }

  // Filter schemas by state - safely handle non-string states
  const getStateString = (state) => {
    if (typeof state === 'string') return state.toLowerCase()
    if (typeof state === 'object' && state !== null) return String(state).toLowerCase()
    return String(state || '').toLowerCase()
  }
  
  const availableSchemas = allSchemas.filter(
    (schema) => getStateString(schema.state) === 'available'
  )

  // Derive approved schemas from the full schema list so newly fetched field
  // details are reflected when a schema is expanded.
  const approvedSchemas = allSchemas.filter(
    (schema) => getStateString(schema.state) === 'approved'
  )

  const blockedSchemas = allSchemas.filter(
    (schema) => getStateString(schema.state) === 'blocked'
  )

  return (
    <div className="p-6 space-y-6">
      {/* Available Schemas Dropdown */}
      <div>
        <h3 className="text-lg font-medium text-gray-900 mb-4">Available Schemas</h3>
        <div className="border rounded-lg bg-white shadow-sm">
          <details className="group">
            <summary className="flex items-center justify-between p-4 cursor-pointer hover:bg-gray-50">
              <span className="font-medium text-gray-900">
                Available Schemas ({availableSchemas.length})
              </span>
              <ChevronRightIcon className="h-5 w-5 text-gray-400 group-open:rotate-90 transition-transform" />
            </summary>
            <div className="border-t bg-gray-50">
              {availableSchemas.length === 0 ? (
                <div className="p-4 text-gray-500 text-center">No available schemas</div>
              ) : (
                <div className="space-y-2 p-4">
                  {availableSchemas.map(schema => {
                    const schemaRangeInfo = schema.fields ? getRangeSchemaInfo(schema) : null
                    return (
                      <div key={schema.name} className="flex items-center justify-between p-3 bg-white rounded border">
                        <div className="flex items-center space-x-3">
                          <div>
                            <h4 className="font-medium text-gray-900">{schema.name}</h4>
                          </div>
                          <span className={`px-2 py-1 rounded-full text-xs font-medium ${getStateColor(schema.state)}`}>
                            {schema.state}
                          </span>
                          {schemaRangeInfo && (
                            <span className="px-2 py-1 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
                              Range Schema
                            </span>
                          )}
                        </div>
                      
                      <div className="flex space-x-2">
                        <button
                          onClick={() => approveSchema(schema.name)}
                          className="px-3 py-1 bg-green-500 text-white rounded text-sm hover:bg-green-600"
                        >
                          Approve
                        </button>
                      </div>
                    </div>
                  )})}
                </div>
              )}
            </div>
          </details>
        </div>
        
      </div>

      {/* Approved Schemas List */}
      <div className="space-y-4">
        <h3 className="text-lg font-medium text-gray-900">Approved Schemas</h3>
        {approvedSchemas.length > 0 ? (
          approvedSchemas.map(renderSchema)
        ) : (
          <div className="border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500">
            No approved schemas. Approve schemas from the available list above to see them here.
          </div>
        )}
      </div>

      {/* Blocked Schemas (if any) */}
      {blockedSchemas.length > 0 && (
        <div className="space-y-4">
          <h3 className="text-lg font-medium text-gray-900">Blocked Schemas</h3>
          {blockedSchemas.map(renderSchema)}
        </div>
      )}
    </div>
  )
}

export default SchemaTab