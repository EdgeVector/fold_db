import { useState } from 'react'
import SchemaSelector from './mutation/SchemaSelector'
import MutationEditor from './mutation/MutationEditor'
import ResultViewer from './mutation/ResultViewer'
import { MutationClient } from '../../api/mutationClient'
import {
  isRangeSchema,
  formatEnhancedRangeSchemaMutation,
  validateRangeKeyForMutation,
  getRangeKey,
  getNonRangeKeyFields
} from '../../utils/rangeSchemaUtils'

function MutationTab({ schemas, onResult }) {
  const [selectedSchema, setSelectedSchema] = useState('')
  const [mutationData, setMutationData] = useState({})
  const [mutationType, setMutationType] = useState('Create')
  const [result, setResult] = useState(null)
  const [rangeKeyValue, setRangeKeyValue] = useState('')

  const handleSchemaChange = (schemaName) => {
    setSelectedSchema(schemaName)
    setMutationData({})
    setRangeKeyValue('')
  }

  const handleFieldChange = (fieldName, value) => {
    setMutationData(prev => ({ ...prev, [fieldName]: value }))
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
      const response = await MutationClient.executeMutation(mutation)
      
      if (!response.success) {
        throw new Error(response.error || 'Mutation failed')
      }
      
      const data = response
      
      if (!response.ok) {
        const errData = { error: data.error || `Mutation failed with status ${response.status}`, status: response.status, details: data }
        setResult(errData)
        onResult(errData)
        return
      }
      
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
          schemas={schemas}
          selectedSchema={selectedSchema}
          mutationType={mutationType}
          onSchemaChange={handleSchemaChange}
          onTypeChange={setMutationType}
        />

        {selectedSchema && isCurrentSchemaRangeSchema && (
          <div className="bg-yellow-50 rounded-lg p-4">
            <h3 className="text-lg font-medium text-gray-900 mb-4">Range Schema Configuration</h3>
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 mb-2">
                {rangeKey} (Range Key)
                {mutationType !== 'Delete' && <span className="ml-2 text-xs text-red-500">Required</span>}
                {mutationType === 'Delete' && <span className="ml-2 text-xs text-blue-500">Optional for targeting</span>}
              </label>
              <input
                type="text"
                className={`mt-1 block w-full rounded-md shadow-sm sm:text-sm ${mutationType !== 'Delete' && !rangeKeyValue.trim() ? 'border-red-300 focus:ring-red-500 focus:border-red-500' : 'border-gray-300 focus:ring-primary focus:border-primary'}`}
                value={rangeKeyValue}
                onChange={(e) => setRangeKeyValue(e.target.value)}
                placeholder={`Enter ${rangeKey} value`}
                required={mutationType !== 'Delete'}
              />
            </div>
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
            Execute Mutation
          </button>
        </div>
      </form>

      <ResultViewer result={result} />
    </div>
  )
}

export default MutationTab
