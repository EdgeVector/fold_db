import { useState, useEffect } from 'react'
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

  // Fetch schemas when component mounts
  useEffect(() => {
    console.log('🟢 SchemaTab: Fetching schemas on mount')
    dispatch(fetchSchemas({ forceRefresh: true }))
  }, [dispatch])

  // Helper to get display name (descriptive_name if available, otherwise name)
  const getDisplayName = (schema) => schema.descriptive_name || schema.name

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
              <h3 className="font-medium text-gray-900">{getDisplayName(schema)}</h3>
              {schema.descriptive_name && schema.descriptive_name !== schema.name && (
                <span className="text-xs text-gray-500">({schema.name})</span>
              )}
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
  


  // Derive approved schemas from the full schema list so newly fetched field
  // details are reflected when a schema is expanded.
  const approvedSchemas = schemas.filter(
    (schema) => getStateString(schema.state) === 'approved'
  )



  return (
    <div className="p-6 space-y-6">
      {/* Approved Schemas List */}
      <div className="space-y-4">
        <h3 className="text-lg font-medium text-gray-900">Approved Schemas</h3>
        {approvedSchemas.length > 0 ? (
          approvedSchemas.map(renderSchema)
        ) : (
          <div className="border rounded-lg p-8 bg-white shadow-sm text-center text-gray-500">
            No approved schemas found.
          </div>
        )}
      </div>
    </div>
  )
}

export default SchemaTab