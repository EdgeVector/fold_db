import { useState } from 'react'
import SchemaSelector from './mutation/SchemaSelector'
import MutationEditor from './mutation/MutationEditor'
import ResultViewer from './mutation/ResultViewer'
import TextField from '../form/TextField'
import { MutationClient } from '../../api'
import { signPayload } from '../../utils/authenticationWrapper'
import { useFormValidation } from '../../hooks/index.js'
import {
  BUTTON_TEXT,
  FORM_LABELS,
  RANGE_SCHEMA_CONFIG,
  VALIDATION_MESSAGES
} from '../../constants'
import {
  isRangeSchema,
  formatEnhancedRangeSchemaMutation,
  validateRangeKeyForMutation,
  getRangeKey,
  getNonRangeKeyFields
} from '../../utils/rangeSchemaUtils'
import { useAppSelector } from '../../store/hooks'
import { selectApprovedSchemas } from '../../store/schemaSlice'

function MutationTab({ onResult }) {
  // Redux state - TASK-003: Use approved schemas for SCHEMA-002 compliance
  const schemas = useAppSelector(selectApprovedSchemas)
  const [selectedSchema, setSelectedSchema] = useState('')
  const [mutationData, setMutationData] = useState({})
  const [mutationType, setMutationType] = useState('Create')
  const [result, setResult] = useState(null)
  const [rangeKeyValue, setRangeKeyValue] = useState('')

  // Use form validation hook (TASK-001)
  const { validate, errors } = useFormValidation()

  const handleSchemaChange = (schemaName) => {
    setSelectedSchema(schemaName)
    setMutationData({})
    setRangeKeyValue('')
  }

  const handleFieldChange = (fieldName, value) => {
    setMutationData(prev => ({ ...prev, [fieldName]: value }))
  }

  const _handleRangeKeyChange = (e) => {
    const value = e.target.value
    setRangeKeyValue(value)
    
    // Validate range key with debouncing (TASK-001)
    const selectedSchemaObj = schemas.find(s => s.name === selectedSchema)
    if (selectedSchemaObj && isRangeSchema(selectedSchemaObj)) {
      validate('rangeKey', value, [
        { type: 'custom', validator: (val) => validateRangeKeyForMutation(val, mutationType !== 'Delete') }
      ], true) // Enable debouncing
    }
  }

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!selectedSchema) return
    
    const selectedSchemaObj = schemas.find(s => s.name === selectedSchema)
    let mutation

    if (isRangeSchema(selectedSchemaObj)) {
      const rangeKeyError = validateRangeKeyForMutation(rangeKeyValue, mutationType !== 'Delete')
      if (rangeKeyError) {
        const errData = { error: rangeKeyError, details: 'Range key validation failed' }
        setResult(errData)
        onResult(errData)
        return
      }
      if (mutationType !== 'Delete' && Object.keys(mutationData).length === 0 && !rangeKeyValue.trim()) return
      mutation = formatEnhancedRangeSchemaMutation(selectedSchemaObj, mutationType, rangeKeyValue, mutationData)
    } else {
      if (mutationType !== 'Delete' && Object.keys(mutationData).length === 0) return
      mutation = {
        type: 'mutation',
        schema: selectedSchema,
        mutation_type: mutationType.toLowerCase(),
        data: mutationType === 'Delete' ? {} : mutationData
      }
    }

    try {
      // Sign the mutation before sending to the API
      const signedMutation = await signPayload(mutation)
      const response = await MutationClient.executeMutation(signedMutation)
      
      if (!response.success) {
        throw new Error(response.error || 'Mutation failed')
      }
      
      const data = response
      
      // Note: Removed response.ok check since response is ApiResponse, not fetch Response
      // The httpClient already handles HTTP errors and the response.success check above handles failures
      
      setResult(data)
      onResult(data)
      if (data.success) {
        setMutationData({})
        setRangeKeyValue('')
      }
    } catch (error) {
      const errData = { error: `Network error: ${error.message}`, details: error }
      setResult(errData)
      onResult(errData)
    }
  }

  const selectedSchemaObj = selectedSchema ? schemas.find(s => s.name === selectedSchema) : null
  const isCurrentSchemaRangeSchema = selectedSchemaObj ? isRangeSchema(selectedSchemaObj) : false
  const rangeKey = selectedSchemaObj ? getRangeKey(selectedSchemaObj) : null
  const selectedSchemaFields = selectedSchemaObj ? (isCurrentSchemaRangeSchema ? getNonRangeKeyFields(selectedSchemaObj) : selectedSchemaObj.fields || {}) : {}

  return (
    <div className="p-6">
      <form onSubmit={handleSubmit} className="space-y-6">
        <SchemaSelector
          selectedSchema={selectedSchema}
          mutationType={mutationType}
          onSchemaChange={handleSchemaChange}
          onTypeChange={setMutationType}
        />

        {selectedSchema && isCurrentSchemaRangeSchema && (
          <div className={`${RANGE_SCHEMA_CONFIG.backgroundColor} rounded-lg p-4`}>
            <h3 className="text-lg font-medium text-gray-900 mb-4">Range Schema Configuration</h3>
            <TextField
              name="rangeKey"
              label={`${rangeKey} (${RANGE_SCHEMA_CONFIG.label})`}
              value={rangeKeyValue}
              onChange={setRangeKeyValue}
              placeholder={`Enter ${rangeKey} value`}
              required={mutationType !== 'Delete'}
              error={errors.rangeKey}
              helpText={
                mutationType !== 'Delete'
                  ? FORM_LABELS.rangeKeyRequired
                  : FORM_LABELS.rangeKeyOptional
              }
              debounced={true}
            />
          </div>
        )}

        {selectedSchema && (
          <MutationEditor
            fields={selectedSchemaFields}
            mutationType={mutationType}
            mutationData={mutationData}
            onFieldChange={handleFieldChange}
            isRangeSchema={isCurrentSchemaRangeSchema}
          />
        )}

        <div className="flex justify-end pt-4">
          <button
            type="submit"
            className={`inline-flex items-center px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white ${!selectedSchema || (mutationType !== 'Delete' && Object.keys(mutationData).length === 0) || (isCurrentSchemaRangeSchema && mutationType !== 'Delete' && !rangeKeyValue.trim()) ? 'bg-gray-300 cursor-not-allowed' : 'bg-primary hover:bg-primary/90 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary'}`}
            disabled={!selectedSchema || (mutationType !== 'Delete' && Object.keys(mutationData).length === 0) || (isCurrentSchemaRangeSchema && mutationType !== 'Delete' && !rangeKeyValue.trim())}
          >
            {BUTTON_TEXT.executeMutation}
          </button>
        </div>
      </form>

      <ResultViewer result={result} />
    </div>
  )
}

export default MutationTab
