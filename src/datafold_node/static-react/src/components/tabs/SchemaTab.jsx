import { useState } from 'react'
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/solid'
import { getRangeSchemaInfo, getHashRangeSchemaInfo } from '../../utils/rangeSchemaHelpers'
import { useAppSelector, useAppDispatch } from '../../store/hooks'
import {
  selectAllSchemas,
  selectFetchLoading,
  selectFetchError,
  approveSchema as approveSchemaAction,
  blockSchema as blockSchemaAction,
  fetchSchemas
} from '../../store/schemaSlice'
import schemaClient from '../../api/clients/schemaClient'
import TopologyDisplay from '../schema/TopologyDisplay'

function SchemaTab({ onResult, onSchemaUpdated }) {
  // Redux state and dispatch - TASK-003: Use Redux instead of props
  const dispatch = useAppDispatch()
  const schemas = useAppSelector(selectAllSchemas)
  const _isLoadingSchemas = useAppSelector(selectFetchLoading)
  const _schemasError = useAppSelector(selectFetchError)
  const [expandedSchemas, setExpandedSchemas] = useState({})

  // Debug logging
  console.log('🟢 SchemaTab: Current schemas from Redux:', schemas.map(s => ({ name: s.name, state: s.state })))



  const toggleSchema = async (schemaName) => {
    const isCurrentlyExpanded = expandedSchemas[schemaName]
    
    setExpandedSchemas(prev => ({
      ...prev,
      [schemaName]: !prev[schemaName]
    }))

    // If expanding and schema doesn't have fields yet, fetch them
    if (!isCurrentlyExpanded) {
      const schema = schemas.find(s => s.name === schemaName)
      if (schema && (!schema.fields || Object.keys(schema.fields).length === 0)) {
        try {
          const response = await schemaClient.getSchema(schemaName)
          if (response.success) {
            // Refresh the schema list to get updated details
            dispatch(fetchSchemas({ forceRefresh: true }))
            if (onSchemaUpdated) {
              onSchemaUpdated()
            }
          }
        } catch (err) {
          console.error(`Failed to fetch schema details for ${schemaName}:`, err)
        }
      }
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
            {field.molecule_uuid && (
              <div className="text-xs text-gray-500 break-all">
                {field.molecule_uuid}
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
    console.log('🟡 SchemaTab: Starting approveSchema for:', schemaName)
    try {
      // Use Redux action instead of direct API call
      const result = await dispatch(approveSchemaAction({ schemaName }))
      console.log('🟡 SchemaTab: approveSchema result:', result)
      
      if (approveSchemaAction.fulfilled.match(result)) {
        console.log('🟡 SchemaTab: approveSchema fulfilled, calling callbacks')
        
        // Extract backfill hash if present
        const backfillHash = result.payload?.backfillHash
        console.log('🔄 Backfill hash:', backfillHash)
        
        // Refetch schemas from backend to get updated states
        console.log('🔄 Refetching schemas from backend after approval...')
        await dispatch(fetchSchemas({ forceRefresh: true }))
        console.log('✅ Refetch complete - backend state should be reflected')
        
        if (onResult) {
          const message = backfillHash 
            ? `Schema ${schemaName} approved successfully. Backfill started with hash: ${backfillHash}` 
            : `Schema ${schemaName} approved successfully`
          onResult({ success: true, message, backfillHash })
        }
        if (onSchemaUpdated) {
          onSchemaUpdated()
        }
      } else {
        console.log('🔴 SchemaTab: approveSchema rejected:', result.payload)
        const errorMessage = typeof result.payload === 'string' 
          ? result.payload 
          : result.payload?.error || `Failed to approve schema: ${schemaName}`
        throw new Error(errorMessage)
      }
    } catch (err) {
      console.error('🔴 SchemaTab: Failed to approve schema:', err)
      if (onResult) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        onResult({ error: `Failed to approve schema: ${errorMessage}` })
      }
    }
  }

  const blockSchema = async (schemaName) => {
    try {
      // Use Redux action instead of direct API call
      const result = await dispatch(blockSchemaAction({ schemaName }))
      
      if (blockSchemaAction.fulfilled.match(result)) {
        console.log('🟡 SchemaTab: blockSchema fulfilled, calling callbacks')
        
        // Refetch schemas from backend to get updated states
        console.log('🔄 Refetching schemas from backend after blocking...')
        await dispatch(fetchSchemas({ forceRefresh: true }))
        console.log('✅ Refetch complete - backend state should be reflected')
        
        if (onResult) {
          onResult({ success: true, message: `Schema ${schemaName} blocked successfully` })
        }
        if (onSchemaUpdated) {
          onSchemaUpdated()
        }
      } else {
        const errorMessage = typeof result.payload === 'string' 
          ? result.payload 
          : result.payload?.error || `Failed to block schema: ${schemaName}`
        throw new Error(errorMessage)
      }
    } catch (err) {
      console.error('Failed to block schema:', err)
      if (onResult) {
        const errorMessage = err instanceof Error ? err.message : String(err)
        onResult({ error: `Failed to block schema: ${errorMessage}` })
      }
    }
  }


  const renderSchema = (schema) => {
    const isExpanded = expandedSchemas[schema.name]
    const state = schema.state || 'Unknown'
    const rangeSchemaInfo = schema.fields ? getRangeSchemaInfo(schema) : null
    const hashRangeSchemaInfo = getHashRangeSchemaInfo(schema)

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
              {hashRangeSchemaInfo && (
                <span className="px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800">
                  HashRange Schema
                </span>
              )}
            </div>
            <div className="flex items-center space-x-2">
              {/* Schema State Transition Logic (SCHEMA-001):
                  - available → approved
                  - approved → blocked (once approved, cannot be unloaded)
                  - blocked → approved (once approved, cannot be unloaded) */}
              {state.toLowerCase() === 'available' && (
                <button
                  className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-green-600 hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-green-500"
                  onClick={(e) => {
                    console.log('🟠 Button clicked: Approve for schema:', schema.name)
                    e.stopPropagation()
                    approveSchema(schema.name)
                  }}
                >
                  Approve
                </button>
              )}
              {state.toLowerCase() === 'approved' && (
                <button
                  className="group inline-flex items-center px-2 py-1 text-xs font-medium rounded-md text-white bg-red-600 hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-red-500"
                  onClick={(e) => {
                    e.stopPropagation()
                    blockSchema(schema.name)
                  }}
                >
                  Block
                </button>
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
            
            {/* HashRange Schema Information */}
            {hashRangeSchemaInfo && (
              <div className="mb-4 p-3 bg-blue-50 rounded-md border border-blue-200">
                <h4 className="text-sm font-medium text-blue-900 mb-2">HashRange Schema Information</h4>
                <div className="space-y-1 text-xs text-blue-800">
                  <p><strong>Hash Field:</strong> {hashRangeSchemaInfo.hashField}</p>
                  <p><strong>Range Field:</strong> {hashRangeSchemaInfo.rangeField}</p>
                  <p><strong>Total Fields:</strong> {hashRangeSchemaInfo.totalFields}</p>
                  <p className="text-blue-600">
                    This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys.
                  </p>
                </div>
              </div>
            )}
            
            <div className="space-y-3">
              {/* Declarative schema: fields is an array of strings */}
              {Array.isArray(schema.fields) ? (
                schema.fields.map(fieldName => {
                  const fieldTopology = schema.field_topologies?.[fieldName]
                  return (
                    <div key={fieldName} className="p-3 bg-gray-50 rounded-md border border-gray-200">
                      <div className="flex items-center justify-between">
                        <div className="flex-1">
                          <div className="flex items-center space-x-2">
                            <span className="font-medium text-gray-900">{fieldName}</span>
                            {rangeSchemaInfo?.rangeKey === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
                                Range Key
                              </span>
                            )}
                            {hashRangeSchemaInfo?.hashField === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-blue-100 text-blue-800">
                                Hash Key
                              </span>
                            )}
                            {hashRangeSchemaInfo?.rangeField === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full bg-purple-100 text-purple-800">
                                Range Key
                              </span>
                            )}
                          </div>
                          {fieldTopology && (
                            <TopologyDisplay topology={fieldTopology} />
                          )}
                        </div>
                      </div>
                    </div>
                  )
                })
              ) : (
                <p className="text-sm text-gray-500 italic">No fields defined</p>
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
  
  const availableSchemas = schemas.filter(
    (schema) => getStateString(schema.state) === 'available'
  )

  // Derive approved schemas from the full schema list so newly fetched field
  // details are reflected when a schema is expanded.
  const approvedSchemas = schemas.filter(
    (schema) => getStateString(schema.state) === 'approved'
  )

  const blockedSchemas = schemas.filter(
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
                    const schemaHashRangeInfo = getHashRangeSchemaInfo(schema)
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
                          {schemaHashRangeInfo && (
                            <span className="px-2 py-1 text-xs font-medium rounded-full bg-blue-100 text-blue-800">
                              HashRange Schema
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