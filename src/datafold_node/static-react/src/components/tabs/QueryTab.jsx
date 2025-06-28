import { useState } from 'react'
import { useRangeSchema, useFormValidation } from '../../hooks/index.js'
import SelectField from '../form/SelectField'
import RangeField from '../form/RangeField'
import { post } from '../../utils/httpClient'
import { API_ENDPOINTS } from '../../api/endpoints'
import { API_CONFIG } from '../../constants/api'
import {
  BUTTON_TEXT,
  FORM_LABELS,
  UI_STATES,
  SCHEMA_STATES,
  VALIDATION_MESSAGES
} from '../../constants'
import { useAppSelector } from '../../store/hooks'
import { selectAllSchemas, selectFetchLoading } from '../../store/schemaSlice'

function QueryTab({ onResult }) {
  // Redux state - TASK-003: Use Redux instead of props
  const schemas = useAppSelector(selectAllSchemas)
  const schemasLoading = useAppSelector(selectFetchLoading)
  const [selectedSchema, setSelectedSchema] = useState('')
  const [queryFields, setQueryFields] = useState([])
  const [rangeFilters, setRangeFilters] = useState({})
  const [, setRangeKeyValue] = useState('')
  const [rangeSchemaFilter, setRangeSchemaFilter] = useState({})

  // Use custom hooks (TASK-001)
  const { rangeProps } = useRangeSchema()
  const { validate, errors, clearErrors } = useFormValidation()

  const handleSchemaChange = (e) => {
    const schemaName = e.target.value
    setSelectedSchema(schemaName)
    
    // Default to all fields being checked when a schema is selected
    if (schemaName) {
      const selectedSchemaObj = schemas.find(s => s.name === schemaName)
      const allFieldNames = selectedSchemaObj?.fields ? Object.keys(selectedSchemaObj.fields) : []
      setQueryFields(allFieldNames)
    } else {
      setQueryFields([])
    }
    
    setRangeFilters({})
    setRangeKeyValue('')
    setRangeSchemaFilter({})
    clearErrors() // Clear validation errors when schema changes
  }

  const handleFieldToggle = (fieldName) => {
    setQueryFields(prev => {
      if (prev.includes(fieldName)) {
        return prev.filter(f => f !== fieldName)
      }
      return [...prev, fieldName]
    })
  }

  const handleRangeFilterChange = (fieldName, filterType, value) => {
    setRangeFilters(prev => ({
      ...prev,
      [fieldName]: {
        ...prev[fieldName],
        [filterType]: value
      }
    }))
  }

  const handleSubmit = async (e) => {
    e.preventDefault()
    
    if (!selectedSchema || queryFields.length === 0) {
      return
    }

    const selectedSchemaObj = schemas.find(s => s.name === selectedSchema)
    let query

    // Check if this is a range schema using the hook
    if (rangeProps.isRange(selectedSchemaObj)) {
      // For range schemas, use the enhanced range filtering
      query = {
        type: 'query',
        schema: selectedSchema,
        fields: queryFields
      }

      // Add range filter based on the enhanced filter options
      if (rangeSchemaFilter.start && rangeSchemaFilter.end) {
        query.filter = {
          range_filter: {
            [rangeProps.getRangeKey(selectedSchemaObj)]: {
              KeyRange: {
                start: rangeSchemaFilter.start,
                end: rangeSchemaFilter.end
              }
            }
          }
        }
      } else if (rangeSchemaFilter.key) {
        query.filter = {
          range_filter: {
            [rangeProps.getRangeKey(selectedSchemaObj)]: rangeSchemaFilter.key
          }
        }
      } else if (rangeSchemaFilter.keyPrefix) {
        query.filter = {
          range_filter: {
            [rangeProps.getRangeKey(selectedSchemaObj)]: {
              KeyPrefix: rangeSchemaFilter.keyPrefix
            }
          }
        }
      }
    } else {
      // For regular schemas, use the existing logic
      query = {
        type: 'query',
        schema: selectedSchema,
        fields: queryFields
      }

      // Add range filters if any are specified for regular schemas
      const selectedSchemaFields = selectedSchemaObj?.fields || {}
      const rangeFieldsWithFilters = queryFields.filter(fieldName => {
        const field = selectedSchemaFields[fieldName]
        return field?.field_type === 'Range' && rangeFilters[fieldName]
      })

      if (rangeFieldsWithFilters.length > 0) {
        const fieldName = rangeFieldsWithFilters[0] // For now, support one range filter
        const filter = rangeFilters[fieldName]
        
        if (filter.start && filter.end) {
          query.filter = {
            field: fieldName,
            range_filter: {
              KeyRange: {
                start: filter.start,
                end: filter.end
              }
            }
          }
        } else if (filter.key) {
          query.filter = {
            field: fieldName,
            range_filter: {
              Key: filter.key
            }
          }
        } else if (filter.keyPrefix) {
          query.filter = {
            field: fieldName,
            range_filter: {
              KeyPrefix: filter.keyPrefix
            }
          }
        }
      }
    }

    try {
      // Use the standardized HTTP client with proper base URL
      const response = await post(API_CONFIG.BASE_URL, API_ENDPOINTS.QUERY, query)
      
      if (!response.success) {
        console.error('Query failed:', response.error)
        onResult({
          error: response.error || 'Query execution failed',
          details: response
        })
        return
      }
      
      onResult(response)
    } catch (error) {
      console.error('Failed to execute query:', error)
      onResult({
        error: `Network error: ${error.message}`,
        details: error
      })
    }
  }

  const selectedSchemaObj = selectedSchema ?
    schemas.find(s => s.name === selectedSchema) : null

  const selectedSchemaFields = selectedSchemaObj?.fields || {}

  const isCurrentSchemaRangeSchema = selectedSchemaObj ? rangeProps.isRange(selectedSchemaObj) : false
  const rangeKey = selectedSchemaObj ? rangeProps.getRangeKey(selectedSchemaObj) : null

  const rangeFields = selectedSchema ?
    Object.entries(selectedSchemaFields).filter(([_, field]) => field.field_type === 'Range') :
    []

  // Filter schemas to only include approved ones (SCHEMA-002)
  // Note: This filtering logic should now be handled by the parent component using useApprovedSchemas hook
  // but we keep this as fallback for backward compatibility
  const approvedSchemas = schemas.filter(schema => {
    // Handle different state formats
    const state = typeof schema.state === 'string'
      ? schema.state.toLowerCase()
      : String(schema.state || '').toLowerCase()
    return state === SCHEMA_STATES.APPROVED
  })

  return (
    <div className="p-6">
      <form onSubmit={handleSubmit} className="space-y-6">
        <SelectField
          name="schema"
          label={FORM_LABELS.schema}
          value={selectedSchema}
          onChange={(value) => {
            const event = { target: { value } };
            handleSchemaChange(event);
          }}
          options={approvedSchemas.map(schema => ({
            value: schema.name,
            label: schema.name
          }))}
          placeholder="Select a schema..."
          emptyMessage={FORM_LABELS.schemaEmpty}
          helpText={FORM_LABELS.schemaHelp}
          loading={schemasLoading}
        />

        {selectedSchema && (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Select Fields
            </label>
            <div className="bg-gray-50 rounded-md p-4">
              <div className="space-y-3">
                {Object.entries(selectedSchemaFields).map(([fieldName, field]) => (
                  <label key={fieldName} className="relative flex items-start">
                    <div className="flex items-center h-5">
                      <input
                        type="checkbox"
                        className="h-4 w-4 text-primary border-gray-300 rounded focus:ring-primary"
                        checked={queryFields.includes(fieldName)}
                        onChange={() => handleFieldToggle(fieldName)}
                      />
                    </div>
                    <div className="ml-3 flex items-center">
                      <span className="text-sm font-medium text-gray-700">{fieldName}</span>
                      <span className="ml-2 inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-gray-100 text-gray-600">
                        {field.field_type}
                      </span>
                    </div>
                  </label>
                ))}
              </div>
            </div>
          </div>
        )}

        {/* Range Schema Filter - only show for range schemas */}
        {isCurrentSchemaRangeSchema && (
          <RangeField
            name="rangeSchemaFilter"
            label={FORM_LABELS.rangeKeyFilter}
            value={rangeSchemaFilter}
            onChange={setRangeSchemaFilter}
            rangeKeyName={rangeKey}
            mode="all"
          />
        )}

        {/* Regular Range Field Filters - only show for non-range schemas */}
        {!isCurrentSchemaRangeSchema && rangeFields.length > 0 && queryFields.some(fieldName =>
          selectedSchemaFields[fieldName]?.field_type === 'Range'
        ) && (
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              Range Field Filters
            </label>
            <div className="bg-blue-50 rounded-md p-4 space-y-4">
              {rangeFields
                .filter(([fieldName]) => queryFields.includes(fieldName))
                .map(([fieldName]) => (
                  <div key={fieldName} className="border-b border-blue-200 pb-4 last:border-b-0 last:pb-0">
                    <h4 className="text-sm font-medium text-gray-800 mb-3">{fieldName}</h4>
                    
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                      {/* Key Range Filter */}
                      <div className="space-y-2">
                        <label className="block text-xs font-medium text-gray-600">Key Range</label>
                        <input
                          type="text"
                          placeholder="Start key"
                          className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                          value={rangeFilters[fieldName]?.start || ''}
                          onChange={(e) => handleRangeFilterChange(fieldName, 'start', e.target.value)}
                        />
                        <input
                          type="text"
                          placeholder="End key"
                          className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                          value={rangeFilters[fieldName]?.end || ''}
                          onChange={(e) => handleRangeFilterChange(fieldName, 'end', e.target.value)}
                        />
                      </div>

                      {/* Single Key Filter */}
                      <div className="space-y-2">
                        <label className="block text-xs font-medium text-gray-600">Exact Key</label>
                        <input
                          type="text"
                          placeholder="Exact key to match"
                          className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                          value={rangeFilters[fieldName]?.key || ''}
                          onChange={(e) => handleRangeFilterChange(fieldName, 'key', e.target.value)}
                        />
                      </div>

                      {/* Key Prefix Filter */}
                      <div className="space-y-2">
                        <label className="block text-xs font-medium text-gray-600">Key Prefix</label>
                        <input
                          type="text"
                          placeholder="Key prefix (e.g., 'user:')"
                          className="w-full px-3 py-2 text-sm border border-gray-300 rounded-md focus:outline-none focus:ring-1 focus:ring-primary focus:border-primary"
                          value={rangeFilters[fieldName]?.keyPrefix || ''}
                          onChange={(e) => handleRangeFilterChange(fieldName, 'keyPrefix', e.target.value)}
                        />
                      </div>
                    </div>

                    <div className="mt-3 text-xs text-gray-500">
                      <p><strong>Key Range:</strong> Matches keys between start and end (inclusive start, exclusive end)</p>
                      <p><strong>Exact Key:</strong> Matches a specific key exactly</p>
                      <p><strong>Key Prefix:</strong> Matches all keys starting with the prefix</p>
                    </div>
                  </div>
                ))}
            </div>
          </div>
        )}

        <div className="flex justify-end">
          <button
            type="submit"
            className={`
              inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white
              ${!selectedSchema || queryFields.length === 0
                ? 'bg-gray-300 cursor-not-allowed'
                : 'bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary'
              }
            `}
            disabled={!selectedSchema || queryFields.length === 0}
          >
            {BUTTON_TEXT.executeQuery}
          </button>
        </div>
      </form>
    </div>
  )
}

export default QueryTab