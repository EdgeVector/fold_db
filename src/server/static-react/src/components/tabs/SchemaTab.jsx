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
import { SCHEMA_BADGE_COLORS } from '../../constants/ui'

function SchemaTab({ onResult, onSchemaUpdated }) {
  // Redux state and dispatch - TASK-003: Use Redux instead of props
  const dispatch = useAppDispatch()
  const schemas = useAppSelector(selectAllSchemas)
  const _isLoadingSchemas = useAppSelector(selectFetchLoading)
  const _schemasError = useAppSelector(selectFetchError)
  const [expandedSchemas, setExpandedSchemas] = useState({})

  // Fetch schemas when component mounts
  useEffect(() => {
    dispatch(fetchSchemas({ forceRefresh: true }))
  }, [dispatch])

  // Helper to get display name (descriptive_name if available, otherwise name)
  const getDisplayName = (schema) => schema.descriptive_name || schema.name

  // Debug logging



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
    const key = state?.toLowerCase()
    return SCHEMA_BADGE_COLORS[key] || 'minimal-badge'
  }

  const approveSchema = async (schemaName) => {
    try {
      // Use Redux action instead of direct API call
      const result = await dispatch(approveSchemaAction({ schemaName }))
      
      if (approveSchemaAction.fulfilled.match(result)) {
        
        // Extract backfill hash if present
        const backfillHash = result.payload?.backfillHash
        
        // Refetch schemas from backend to get updated states
        await dispatch(fetchSchemas({ forceRefresh: true }))
        
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
        
        // Refetch schemas from backend to get updated states
        await dispatch(fetchSchemas({ forceRefresh: true }))
        
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
      <div key={schema.name} className="minimal-card overflow-hidden transition-all duration-200 hover:shadow-md">
        <div
          className="px-4 py-3 bg-surface-secondary cursor-pointer select-none transition-colors duration-200 hover:bg-surface-secondary"
          onClick={() => toggleSchema(schema.name)}
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              {isExpanded ? (
                <ChevronDownIcon className="w-4 h-4 text-tertiary transition-transform duration-200" />
              ) : (
                <ChevronRightIcon className="w-4 h-4 text-tertiary transition-transform duration-200" />
              )}
              <h3 className="font-medium text-primary">{getDisplayName(schema)}</h3>
              {schema.descriptive_name && schema.descriptive_name !== schema.name && (
                <span className="text-xs text-secondary" title={schema.name}>({schema.name.length > 12 ? schema.name.slice(0, 8) + '…' : schema.name})</span>
              )}
              <span className={`px-2 py-1 text-xs font-medium rounded-full ${getStateColor(state)}`}>
                {state}
              </span>
              {rangeSchemaInfo && (
                <span className="px-2 py-1 text-xs font-medium rounded-full minimal-section-purple-text" style={{background: '#f3e8ff'}}>
                  Range Schema
                </span>
              )}
              {hashRangeSchemaInfo && (
                <span className="px-2 py-1 text-xs font-medium rounded-full minimal-section-info-text" style={{background: '#dbeafe'}}>
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
                  className="minimal-btn-secondary text-xs py-1 px-3"
                  onClick={(e) => {
                    e.stopPropagation()
                    approveSchema(schema.name)
                  }}
                >
                  Approve
                </button>
              )}
              {state.toLowerCase() === 'approved' && (
                <button
                  className="minimal-btn-secondary text-xs py-1 px-3 hover:text-error"
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
                  className="minimal-btn-secondary text-xs py-1 px-3"
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
          <div className="p-4 border-t" style={{borderColor: 'var(--color-border)'}}>
            {/* Range Schema Information */}
            {rangeSchemaInfo && (
              <div className="mb-4 p-3 minimal-section-purple">
                <h4 className="text-sm font-medium minimal-section-purple-text mb-2">Range Schema Information</h4>
                <div className="space-y-1 text-xs minimal-section-purple-text">
                  <p><strong>Range Key:</strong> {rangeSchemaInfo.rangeKey}</p>
                  <p><strong>Total Fields:</strong> {rangeSchemaInfo.totalFields}</p>
                  <p><strong>Range Fields:</strong> {rangeSchemaInfo.rangeFields.length}</p>
                  <p className="minimal-section-purple-muted">
                    This schema uses range-based storage for efficient querying and mutations.
                  </p>
                </div>
              </div>
            )}
            
            {/* HashRange Schema Information */}
            {hashRangeSchemaInfo && (
              <div className="mb-4 p-3 minimal-section-info">
                <h4 className="text-sm font-medium minimal-section-info-text mb-2">HashRange Schema Information</h4>
                <div className="space-y-1 text-xs minimal-section-info-text">
                  <p><strong>Hash Field:</strong> {hashRangeSchemaInfo.hashField}</p>
                  <p><strong>Range Field:</strong> {hashRangeSchemaInfo.rangeField}</p>
                  <p><strong>Total Fields:</strong> {hashRangeSchemaInfo.totalFields}</p>
                  <p className="minimal-section-info-muted">
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
                    <div key={fieldName} className="p-3 minimal-card">
                      <div className="flex items-center justify-between">
                        <div className="flex-1">
                          <div className="flex items-center space-x-2">
                            <span className="font-medium text-primary">{fieldName}</span>
                            {rangeSchemaInfo?.rangeKey === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full minimal-section-purple-text" style={{background: '#f3e8ff'}}>
                                Range Key
                              </span>
                            )}
                            {hashRangeSchemaInfo?.hashField === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full minimal-section-info-text" style={{background: '#dbeafe'}}>
                                Hash Key
                              </span>
                            )}
                            {hashRangeSchemaInfo?.rangeField === fieldName && (
                              <span className="px-2 py-0.5 text-xs font-medium rounded-full minimal-section-purple-text" style={{background: '#f3e8ff'}}>
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
                <p className="text-sm text-secondary italic">No fields defined</p>
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
        <h3 className="text-lg font-medium text-primary">Approved Schemas</h3>
        {approvedSchemas.length > 0 ? (
          approvedSchemas.map(renderSchema)
        ) : (
          <div className="minimal-card p-8 text-center text-secondary">
            No approved schemas found.
          </div>
        )}
      </div>
    </div>
  )
}

export default SchemaTab