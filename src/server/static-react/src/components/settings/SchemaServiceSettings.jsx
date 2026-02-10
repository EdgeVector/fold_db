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
      <p className="text-sm text-secondary mb-4">
        The schema service provides centralized schema management and prevents duplicate schemas.
      </p>

      <div className="card p-4">
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm font-medium text-secondary">Backend Configuration</span>
          {schemaServiceLoading ? (
            <span className="badge badge-neutral flex items-center gap-1">
              <span className="spinner w-3 h-3" />
              Loading...
            </span>
          ) : (
            <button onClick={loadSchemaServiceStatus} className="btn-secondary btn-sm">
              Refresh
            </button>
          )}
        </div>

        {schemaServiceUrl ? (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="badge badge-success">✓ Connected</span>
              <span className="text-sm text-primary">Remote Schema Service</span>
            </div>
            <p className="text-xs text-secondary font-mono break-all">
              {schemaServiceUrl}
            </p>
          </div>
        ) : (
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <span className="badge badge-neutral">○ Local</span>
              <span className="text-sm text-primary">Embedded Schema Storage</span>
            </div>
            <p className="text-xs text-secondary">
              Schemas are stored locally. No remote schema service configured.
            </p>
          </div>
        )}
      </div>

      <div className="card card-info p-3">
        <p className="text-xs text-secondary">
          <strong>Note:</strong> Schema service configuration is set at server startup via the <code className="text-info">--schema-service-url</code> flag or environment variable.
        </p>
      </div>
    </div>
  )
}

export default SchemaServiceSettings
