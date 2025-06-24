
function SchemaSelector({ schemas, selectedSchema, mutationType, onSchemaChange, onTypeChange }) {
  // Filter schemas to only include approved ones (SCHEMA-002)
  const approvedSchemas = schemas.filter(schema => {
    // Handle different state formats
    const state = typeof schema.state === 'string'
      ? schema.state.toLowerCase()
      : String(schema.state || '').toLowerCase()
    return state === 'approved'
  })

  return (
    <div className="grid grid-cols-2 gap-4">
      <div>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          Schema
        </label>
        {approvedSchemas.length === 0 ? (
          <div className="mt-1 block w-full pl-3 pr-10 py-2 text-base bg-gray-100 border border-gray-300 rounded-md text-gray-500">
            No approved schemas available for mutations
          </div>
        ) : (
          <select
            className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-primary focus:border-primary rounded-md"
            value={selectedSchema}
            onChange={(e) => onSchemaChange(e.target.value)}
          >
            <option value="">Select a schema...</option>
            {approvedSchemas.map((schema) => (
              <option key={schema.name} value={schema.name}>
                {schema.name} (approved)
              </option>
            ))}
          </select>
        )}
        <p className="mt-1 text-xs text-gray-500">
          Only approved schemas can be mutated (SCHEMA-002)
        </p>
      </div>

      <div>
        <label className="block text-sm font-medium text-gray-700 mb-2">
          Operation Type
        </label>
        <select
          className="mt-1 block w-full pl-3 pr-10 py-2 text-base border-gray-300 focus:outline-none focus:ring-primary focus:border-primary rounded-md"
          value={mutationType}
          onChange={(e) => onTypeChange(e.target.value)}
        >
          <option value="Create">Create - Add new data</option>
          <option value="Update">Update - Modify existing data</option>
          <option value="Delete">Delete - Remove existing data</option>
        </select>
        <p className="mt-1 text-xs text-gray-500">Choose the type of mutation to perform</p>
      </div>
    </div>
  )
}

export default SchemaSelector
