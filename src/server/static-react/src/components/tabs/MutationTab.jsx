import { useState } from 'react'
import SchemaSelector from './mutation/SchemaSelector'
import MutationEditor from './mutation/MutationEditor'
import ResultViewer from './mutation/ResultViewer'
import TextField from '../form/TextField'
import { mutationClient } from '../../api'
import { MUTATION_TYPE_API_MAP, BUTTON_TEXT, FORM_LABELS, RANGE_SCHEMA_CONFIG } from '../../constants/ui.js'
import {
  isRangeSchema,
  formatRangeMutation,
  getRangeKey
} from '../../utils/rangeSchemaHelpers'
import { useAppSelector } from '../../store/hooks'
import { selectApprovedSchemas } from '../../store/schemaSlice'

function MutationTab({ onResult }) {
  // Redux state
  const schemas = useAppSelector(selectApprovedSchemas)
  const [selectedSchema, setSelectedSchema] = useState('')
  const [mutationData, setMutationData] = useState({})
  const [mutationType, setMutationType] = useState('Insert')
  const [result, setResult] = useState(null)
  const [rangeKeyValue, setRangeKeyValue] = useState('')


  const handleSchemaChange = (schemaName) => {
    setSelectedSchema(schemaName)
    setMutationData({})
    setMutationType('Insert')
    setRangeKeyValue('')
  }

  const handleFieldChange = (fieldName, value) => {
    setMutationData(prev => ({ ...prev, [fieldName]: value }))
  }

  const handleSubmit = async (e) => {
    e.preventDefault()
    if (!selectedSchema) return
    
    const selectedSchemaObj = schemas.find(s => s.name === selectedSchema)
    const normalizedMutationType = mutationType
      ? (MUTATION_TYPE_API_MAP[mutationType] || mutationType.toLowerCase())
      : ''
    if (!normalizedMutationType) {
      return
    }
    let mutation

    // Backend handles all validation
    if (isRangeSchema(selectedSchemaObj)) {
      mutation = formatRangeMutation(selectedSchemaObj, mutationType, rangeKeyValue, mutationData)
    } else {
      mutation = {
        type: 'mutation',
        schema: selectedSchema,
        mutation_type: normalizedMutationType,
        fields_and_values: mutationType === 'Delete' ? {} : mutationData,
        key_value: { hash: null, range: null }
      }
    }

    try {
      // Send the mutation directly to the API (no signing required)
      const response = await mutationClient.executeMutation(mutation)
      
      if (!response.success) {
        throw new Error(response.error || 'Mutation failed')
      }
      
      const data = response
      
      // Note: Removed response.ok check since response is ApiResponse, not fetch Response
      // The API client already handles HTTP errors and the response.success check above handles failures
      
      setResult(data)
      onResult(data)
      if (data.success) {
        setMutationData({})
        setRangeKeyValue('')
      }
    } catch (error) {
      const errData = { error: `Network error: ${error instanceof Error ? error.message : String(error)}`, details: error }
      setResult(errData)
      onResult(errData)
    }
  }

  const selectedSchemaObj = selectedSchema ? schemas.find(s => s.name === selectedSchema) : null
  const isCurrentSchemaRangeSchema = selectedSchemaObj ? isRangeSchema(selectedSchemaObj) : false
  const rangeKey = selectedSchemaObj ? getRangeKey(selectedSchemaObj) : null
  
  // Convert declarative schema fields array to object format for MutationEditor
  // For range schemas, filter out the range key (it's shown separately in Range Schema Configuration)
  const getFieldsForMutation = () => {
    if (!selectedSchemaObj || !Array.isArray(selectedSchemaObj.fields)) return {}
    
    const fieldsToShow = isCurrentSchemaRangeSchema 
      ? selectedSchemaObj.fields.filter(field => field !== rangeKey)
      : selectedSchemaObj.fields
    
    return fieldsToShow.reduce((acc, fieldName) => {
      acc[fieldName] = {}
      return acc
    }, {})
  }
  
  const selectedSchemaFields = getFieldsForMutation()

  const isMutationDisabled =
    !selectedSchema ||
    !mutationType ||
    (mutationType !== 'Delete' && Object.keys(mutationData).length === 0) ||
    (isCurrentSchemaRangeSchema && mutationType !== 'Delete' && !rangeKeyValue.trim())

  return (
    <div>
      <form onSubmit={handleSubmit} className="space-y-6">
        <SchemaSelector
          selectedSchema={selectedSchema}
          mutationType={mutationType}
          onSchemaChange={handleSchemaChange}
          onTypeChange={setMutationType}
        />

        {selectedSchema && isCurrentSchemaRangeSchema && (
          <div className={RANGE_SCHEMA_CONFIG.backgroundColor}>
            <h3 className="text-lg font-medium text-primary mb-4">Range Schema Configuration</h3>
            <TextField
              name="rangeKey"
              label={`${rangeKey} (${RANGE_SCHEMA_CONFIG.label})`}
              value={rangeKeyValue}
              onChange={setRangeKeyValue}
              placeholder={`Enter ${rangeKey} value`}
              required={mutationType !== 'Delete'}
              error={undefined}
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
          <button type="submit" className="btn-primary btn-lg" disabled={isMutationDisabled}>
            → {BUTTON_TEXT.executeMutation}
          </button>
        </div>
      </form>

      <ResultViewer result={result} />
    </div>
  )
}

export default MutationTab
