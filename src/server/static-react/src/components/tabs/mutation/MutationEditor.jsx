
function MutationEditor({ fields, mutationType, mutationData, onFieldChange, isRangeSchema }) {
  if (mutationType === 'Delete') {
    return (
      <div className="card p-6">
        <h3 className="text-lg font-medium text-primary mb-4">Delete Operation</h3>
        <p className="text-sm text-secondary">
          This will delete the selected schema. No additional fields are required.
        </p>
      </div>
    )
  }

  const renderField = (fieldName, field) => {
    // Fields are writable by default unless explicitly marked as non-writable
    const isWritable = field.writable !== false
    if (!isWritable) return null
    const value = mutationData[fieldName] || ''

    switch (field.field_type) {
      case 'Collection': {
        let arrayValue = []
        if (value) {
          try {
            const parsed = typeof value === 'string' ? JSON.parse(value) : value
            arrayValue = Array.isArray(parsed) ? parsed : [parsed]
          } catch {
            arrayValue = value.trim() ? [value] : []
          }
        }

        return (
          <div key={fieldName} className="mb-6">
            <label className="block text-sm font-medium text-primary mb-2">
              {fieldName}
              <span className="ml-2 text-xs text-secondary">Collection</span>
            </label>
            <textarea
              className="input mt-1 block w-full sm:text-sm font-mono"
              value={arrayValue.length > 0 ? JSON.stringify(arrayValue, null, 2) : ''}
              onChange={(e) => {
                const inputValue = e.target.value.trim()
                if (!inputValue) {
                  onFieldChange(fieldName, [])
                  return
                }
                try {
                  const parsed = JSON.parse(inputValue)
                  onFieldChange(fieldName, Array.isArray(parsed) ? parsed : [parsed])
                } catch {
                  onFieldChange(fieldName, [inputValue])
                }
              }}
              placeholder={'Enter JSON array (e.g., ["item1", "item2"])'}
              rows={4}
            />
            <p className="mt-1 text-xs text-secondary">
              Enter data as a JSON array. Empty input will create an empty array.
            </p>
          </div>
        )
      }
      case 'Range': {
        // For range schemas, treat Range fields as single value inputs
        // The backend will handle converting single values to range format
        if (isRangeSchema) {
          return (
            <div key={fieldName} className="mb-6">
              <label className="block text-sm font-medium text-primary mb-2">
                {fieldName}
                <span className="ml-2 text-xs text-secondary">Single Value (Range Schema)</span>
              </label>
              <input
                type="text"
                className="input mt-1 block w-full sm:text-sm"
                value={value}
                onChange={(e) => onFieldChange(fieldName, e.target.value)}
                placeholder={`Enter ${fieldName} value`}
              />
              <p className="mt-1 text-xs text-secondary">
                Enter a single value. The system will automatically handle range formatting.
              </p>
            </div>
          )
        }

        // For non-range schemas, use the existing complex Range field UI
        let rangeValue = {}
        if (value) {
          try {
            rangeValue = typeof value === 'string' ? JSON.parse(value) : value
            if (typeof rangeValue !== 'object' || Array.isArray(rangeValue)) {
              rangeValue = {}
            }
          } catch {
            rangeValue = {}
          }
        }

        const rangeEntries = Object.entries(rangeValue)

        const addKeyValuePair = () => {
          const newEntries = [...rangeEntries, ['', '']]
          // Don't filter out empty keys immediately - let user type
          const newRangeValue = Object.fromEntries(newEntries)
          onFieldChange(fieldName, newRangeValue)
        }

        const updateKeyValuePair = (index, key, val) => {
          const newEntries = [...rangeEntries]
          newEntries[index] = [key, val]
          // Keep all entries during editing, including empty ones
          const newRangeValue = Object.fromEntries(newEntries)
          onFieldChange(fieldName, newRangeValue)
        }

        const removeKeyValuePair = (index) => {
          const newEntries = rangeEntries.filter((_, i) => i !== index)
          const newRangeValue = Object.fromEntries(newEntries)
          onFieldChange(fieldName, newRangeValue)
        }

        return (
          <div key={fieldName} className="mb-6">
            <label className="block text-sm font-medium text-primary mb-2">
              {fieldName}
              <span className="ml-2 text-xs text-secondary">Range (Complex)</span>
            </label>
            <div className="card p-4">
              <div className="space-y-3">
                {rangeEntries.length === 0 ? (
                  <p className="text-sm text-secondary italic">No key-value pairs added yet</p>
                ) : (
                  rangeEntries.map(([key, val], index) => (
                    <div key={`${index}-${key}`} className="flex items-center space-x-2">
                      <input
                        type="text"
                        placeholder="Key"
                        className="input flex-1 sm:text-sm"
                        value={key}
                        onChange={(e) => updateKeyValuePair(index, e.target.value, val)}
                      />
                      <span className="text-secondary">:</span>
                      <input
                        type="text"
                        placeholder="Value"
                        className="input flex-1 sm:text-sm"
                        value={val}
                        onChange={(e) => updateKeyValuePair(index, key, e.target.value)}
                      />
                      <button
                        type="button"
                        onClick={() => removeKeyValuePair(index)}
                        className="text-gruvbox-red hover:opacity-75 p-1"
                        title="Remove this key-value pair"
                      >
                        <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                        </svg>
                      </button>
                    </div>
                  ))
                )}
                <button
                  type="button"
                  onClick={addKeyValuePair}
                  className="btn-secondary text-sm leading-4"
                >
                  <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                  </svg>
                  Add Key-Value Pair
                </button>
              </div>
            </div>
            <p className="mt-1 text-xs text-secondary">
              Add key-value pairs for this range field. Empty keys will be filtered out.
            </p>
          </div>
        )
      }
      default:
        return (
          <div key={fieldName} className="mb-6">
            <label className="block text-sm font-medium text-primary mb-2">
              {fieldName}
              <span className="ml-2 text-xs text-secondary">Single</span>
            </label>
            <input
              type="text"
              className="input mt-1 block w-full sm:text-sm"
              value={value}
              onChange={(e) => onFieldChange(fieldName, e.target.value)}
              placeholder={`Enter ${fieldName}`}
            />
          </div>
        )
    }
  }

  return (
    <div className="card p-6">
      <h3 className="text-lg font-medium text-primary mb-4">
        Schema Fields
        {isRangeSchema && (
          <span className="ml-2 text-sm text-gruvbox-blue font-normal">
            (Range Schema - Single Values)
          </span>
        )}
      </h3>
      <div className="space-y-6">
        {Object.entries(fields).map(([name, field]) => renderField(name, field))}
      </div>
      {isRangeSchema && Object.keys(fields).length === 0 && (
        <p className="text-sm text-secondary italic">
          No additional fields to configure. Only the range key is required for this schema.
        </p>
      )}
    </div>
  )
}

export default MutationEditor
