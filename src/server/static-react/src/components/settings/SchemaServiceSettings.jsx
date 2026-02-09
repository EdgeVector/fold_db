import { useState, useEffect } from 'react'
import { getSystemStatus } from '../../api/clients/systemClient'

function SchemaServiceSettings() {
  const [schemaServiceUrl, setSchemaServiceUrl] = useState(null)
  const [schemaServiceLoading, setSchemaServiceLoading] = useState(false)

  useEffect(() => {
    loadSchemaServiceStatus()
  }, [])

  const loadSchemaServiceStatus = async () => {
    setSchemaServiceLoading(true)
    try {
      const response = await getSystemStatus()
      if (response.success && response.data) {
        setSchemaServiceUrl(response.data.schema_service_url || null)
      }
    } catch (error) {
      console.error('Failed to load schema service status:', error)
    } finally {
      setSchemaServiceLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <div className="mb-4">
        <h4 className="text-md font-semibold text-terminal-green mb-2"># Schema Service</h4>
        <p className="text-sm text-terminal-dim mb-4">
          The schema service provides centralized schema management and prevents duplicate schemas.
        </p>
      </div>

      <div className="p-4 border border-terminal card-terminal">
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm font-medium text-terminal-dim">Backend Configuration</span>
          {schemaServiceLoading ? (
            <span className="inline-flex items-center text-xs badge-terminal px-2 py-1">
              <svg className="animate-spin h-3 w-3 mr-1" viewBox="0 0 24 24">
                <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
              </svg>
              Loading...
            </span>
          ) : (
            <button
              onClick={loadSchemaServiceStatus}
              className="text-xs text-terminal-blue hover:text-terminal-cyan"
            >
              Refresh
            </button>
          )}
        </div>

        {schemaServiceUrl ? (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="inline-flex items-center text-xs badge-terminal badge-terminal-success px-2 py-1">
                ✓ Connected
              </span>
              <span className="text-sm text-terminal">Remote Schema Service</span>
            </div>
            <p className="text-xs text-terminal-dim font-mono break-all">
              {schemaServiceUrl}
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="inline-flex items-center text-xs badge-terminal px-2 py-1">
                ○ Local
              </span>
              <span className="text-sm text-terminal">Embedded Schema Storage</span>
            </div>
            <p className="text-xs text-terminal-dim">
              Schemas are stored locally. No remote schema service configured.
            </p>
          </div>
        )}
      </div>

      <div className="p-3 border border-terminal-dim card-terminal">
        <p className="text-xs text-terminal-dim">
          <strong>Note:</strong> Schema service configuration is set at server startup via the <code className="text-terminal-cyan">--schema-service-url</code> flag or environment variable.
        </p>
      </div>
    </div>
  )
}

export default SchemaServiceSettings
