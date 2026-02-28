import { useState, useEffect, useRef } from 'react'
import { ChevronDownIcon, ChevronRightIcon } from '@heroicons/react/24/solid'
import { getRangeSchemaInfo, getHashRangeSchemaInfo } from '../../utils/rangeSchemaHelpers'
import { useAppSelector, useAppDispatch } from '../../store/hooks'
import {
  selectAllSchemas,
  selectApprovedSchemas,
  approveSchema as approveSchemaAction,
  blockSchema as blockSchemaAction,
  fetchSchemas
} from '../../store/schemaSlice'
import SchemaName from '../shared/SchemaName'
import { SCHEMA_BADGE_COLORS } from '../../constants/ui'
import { toErrorMessage } from '../../utils/schemaUtils'

function SchemaTab({ onResult, onSchemaUpdated }) {
  const highlightTimerRef = useRef(null)
  // Redux state and dispatch
  const dispatch = useAppDispatch()
  const schemas = useAppSelector(selectAllSchemas)
  const [expandedSchemas, setExpandedSchemas] = useState({})
  const [highlightedSchema, setHighlightedSchema] = useState(null)

  // Fetch schemas when component mounts; clean up highlight timer on unmount
  useEffect(() => {
    dispatch(fetchSchemas({ forceRefresh: true }))
    return () => { if (highlightTimerRef.current) clearTimeout(highlightTimerRef.current) }
  }, [dispatch])

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
        dispatch(fetchSchemas({ forceRefresh: true }))
        if (onSchemaUpdated) {
          onSchemaUpdated()
        }
      }
    }
  }





  const getStateColor = (state) => {
    const key = state?.toLowerCase()
    const colors = {
      approved: 'badge badge-success',
      pending: 'badge badge-warning',
      blocked: 'badge badge-error',
      available: 'badge badge-info'
    }
    return colors[key] || 'bg-surface-secondary text-secondary border border-border'
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
        onResult({ error: `Failed to approve schema: ${toErrorMessage(err)}` })
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
        onResult({ error: `Failed to block schema: ${toErrorMessage(err)}` })
      }
    }
  }


  const scrollToSchema = (schemaName) => {
    // Expand the target schema and highlight it
    setExpandedSchemas(prev => ({ ...prev, [schemaName]: true }))
    setHighlightedSchema(schemaName)
    // Scroll to it after React re-renders
    window.requestAnimationFrame(() => {
      const el = document.getElementById(`schema-${schemaName}`)
      if (el) {
        el.scrollIntoView({ behavior: 'smooth', block: 'start' })
      }
    })
    // Clear highlight after 2 seconds
    if (highlightTimerRef.current) clearTimeout(highlightTimerRef.current)
    highlightTimerRef.current = setTimeout(() => {
      setHighlightedSchema(null)
      highlightTimerRef.current = null
    }, 2000)
  }

  const renderSchema = (schema) => {
    const isExpanded = expandedSchemas[schema.name]
    const state = schema.state || 'Unknown'
    const rangeSchemaInfo = schema.fields ? getRangeSchemaInfo(schema) : null
    const hashRangeSchemaInfo = getHashRangeSchemaInfo(schema)

    return (
      <div key={schema.name} id={`schema-${schema.name}`} className={`card overflow-hidden transition-shadow duration-500${highlightedSchema === schema.name ? ' ring-2 ring-gruvbox-purple' : ''}`}>
        <button
          type="button"
          className="w-full px-4 py-3 bg-surface-secondary cursor-pointer select-none text-left"
          onClick={() => toggleSchema(schema.name)}
          aria-expanded={isExpanded}
          aria-label={`${isExpanded ? 'Collapse' : 'Expand'} schema ${schema.descriptive_name || schema.name}`}
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              {isExpanded ? (
                <ChevronDownIcon className="w-4 h-4 text-tertiary transition-transform duration-200" />
              ) : (
                <ChevronRightIcon className="w-4 h-4 text-tertiary transition-transform duration-200" />
              )}
              <h3 className="font-medium text-primary">
                <SchemaName schema={schema} className="font-medium text-primary" />
              </h3>
              <span className={`px-2 py-1 text-xs font-medium rounded-full ${getStateColor(state)}`}>
                {state}
              </span>
              {rangeSchemaInfo && (
                <span className="badge badge-info">Range Schema</span>
              )}
              {hashRangeSchemaInfo && (
                <span className="badge badge-info">HashRange Schema</span>
              )}
            </div>
            <div className="flex items-center space-x-2">
              {/* Schema State Transition Logic (SCHEMA-001):
                  - available → approved
                  - approved → blocked (once approved, cannot be unloaded)
                  - blocked → approved (once approved, cannot be unloaded) */}
              {state.toLowerCase() === 'available' && (
                <button
                  className="btn-secondary btn-sm"
                  onClick={(e) => { e.stopPropagation(); approveSchema(schema.name) }}
                >
                  Approve
                </button>
              )}
              {state.toLowerCase() === 'approved' && (
                <button
                  className="btn-secondary btn-sm hover:border-gruvbox-red hover:text-gruvbox-red"
                  onClick={(e) => { e.stopPropagation(); blockSchema(schema.name) }}
                >
                  Block
                </button>
              )}
              {state.toLowerCase() === 'blocked' && (
                <button
                  className="btn-secondary btn-sm"
                  onClick={(e) => { e.stopPropagation(); approveSchema(schema.name) }}
                >
                  Re-approve
                </button>
              )}
            </div>
          </div>
        </button>

        {isExpanded && schema.fields && (
          <div className="p-4 border-t border-border">
            {/* Range Schema Information */}
            {rangeSchemaInfo && (
              <div className="card card-info mb-4 p-3">
                <h4 className="text-sm font-medium text-gruvbox-blue mb-2">Range Schema Information</h4>
                <div className="space-y-1 text-xs text-gruvbox-blue">
                  <p><strong>Range Key:</strong> {rangeSchemaInfo.rangeKey}</p>
                  <p><strong>Total Fields:</strong> {rangeSchemaInfo.totalFields}</p>
                  <p><strong>Range Fields:</strong> {rangeSchemaInfo.rangeFields.length}</p>
                  <p className="opacity-70">This schema uses range-based storage for efficient querying and mutations.</p>
                </div>
              </div>
            )}

            {/* HashRange Schema Information */}
            {hashRangeSchemaInfo && (
              <div className="card card-info mb-4 p-3">
                <h4 className="text-sm font-medium text-gruvbox-blue mb-2">HashRange Schema Information</h4>
                <div className="space-y-1 text-xs text-gruvbox-blue">
                  <p><strong>Hash Field:</strong> {hashRangeSchemaInfo.hashField}</p>
                  <p><strong>Range Field:</strong> {hashRangeSchemaInfo.rangeField}</p>
                  <p><strong>Total Fields:</strong> {hashRangeSchemaInfo.totalFields}</p>
                  <p className="opacity-70">This schema uses hash-range-based storage for efficient querying and mutations with both hash and range keys.</p>
                </div>
              </div>
            )}

            <div className="space-y-3">
              {/* Declarative schema: fields is an array of strings */}
              {Array.isArray(schema.fields) ? (
                schema.fields.map(fieldName => {
                  const classifications = schema.field_classifications?.[fieldName]
                  const refSchema = schema.ref_fields?.[fieldName]
                  return (
                    <div key={fieldName} className="card p-3">
                      <div className="flex items-center justify-between">
                        <div className="flex-1">
                          <div className="flex items-center space-x-2">
                            <span className="font-medium text-primary">{fieldName}</span>
                            {rangeSchemaInfo?.rangeKey === fieldName && (
                              <span className="badge badge-info">Range Key</span>
                            )}
                            {hashRangeSchemaInfo?.hashField === fieldName && (
                              <span className="badge badge-info">Hash Key</span>
                            )}
                            {hashRangeSchemaInfo?.rangeField === fieldName && (
                              <span className="badge badge-info">Range Key</span>
                            )}
                            {classifications && classifications.length > 0 && (
                              <span className="flex space-x-1">
                                {classifications.map(cls => (
                                  <span key={cls} className="px-1.5 py-0.5 text-xs bg-surface-secondary text-primary rounded-full font-mono">
                                    {cls}
                                  </span>
                                ))}
                              </span>
                            )}
                            {refSchema && (
                              <button
                                className="font-mono text-xs text-gruvbox-purple hover:text-gruvbox-yellow underline decoration-dotted cursor-pointer bg-transparent border-none p-0"
                                title={refSchema}
                                onClick={() => scrollToSchema(refSchema)}
                              >
                                Ref&lt;{refSchema.length > 16 ? refSchema.slice(0, 12) + '...' : refSchema}&gt;
                              </button>
                            )}
                          </div>
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

  const approvedSchemas = useAppSelector(selectApprovedSchemas)



  return (
    <div className="space-y-4">
      {approvedSchemas.length > 0 ? (
        approvedSchemas.map(renderSchema)
      ) : (
        <p className="text-secondary">No approved schemas found.</p>
      )}
    </div>
  )
}

export default SchemaTab