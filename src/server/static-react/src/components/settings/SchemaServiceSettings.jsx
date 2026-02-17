import { useState, useEffect } from 'react'
import { getSystemStatus, applySetup } from '../../api/clients/systemClient'

function SchemaServiceSettings() {
  const [schemaServiceUrl, setSchemaServiceUrl] = useState(null)
  const [editUrl, setEditUrl] = useState('')
  const [isEditing, setIsEditing] = useState(false)
  const [schemaServiceLoading, setSchemaServiceLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [saveResult, setSaveResult] = useState(null)

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

  const handleEdit = () => {
    setEditUrl(schemaServiceUrl || '')
    setIsEditing(true)
    setSaveResult(null)
  }

  const handleCancel = () => {
    setIsEditing(false)
    setSaveResult(null)
  }

  const handleSave = async () => {
    setSaving(true)
    setSaveResult(null)
    try {
      const response = await applySetup({ schema_service_url: editUrl })
      if (response.success) {
        setSaveResult('success')
        setSchemaServiceUrl(editUrl)
        setIsEditing(false)
      } else {
        setSaveResult('error')
      }
    } catch {
      setSaveResult('error')
    } finally {
      setSaving(false)
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
          <div className="flex items-center gap-2">
            {schemaServiceLoading ? (
              <span className="badge badge-neutral flex items-center gap-1">
                <span className="spinner w-3 h-3" />
                Loading...
              </span>
            ) : (
              <>
                <button onClick={loadSchemaServiceStatus} className="btn-secondary btn-sm">
                  Refresh
                </button>
                {!isEditing && (
                  <button onClick={handleEdit} className="btn-secondary btn-sm">
                    Edit
                  </button>
                )}
              </>
            )}
          </div>
        </div>

        {isEditing ? (
          <div className="space-y-3">
            <div>
              <label className="text-xs text-secondary block mb-1">Schema Service URL</label>
              <input
                type="text"
                value={editUrl}
                onChange={e => setEditUrl(e.target.value)}
                placeholder="https://..."
                className="w-full px-3 py-2 bg-surface border border-border text-primary text-sm font-mono focus:border-gruvbox-blue focus:outline-none"
                data-testid="schema-url-input"
              />
            </div>
            {saveResult === 'error' && (
              <p className="text-xs text-gruvbox-red">Failed to save. Please try again.</p>
            )}
            <div className="flex gap-2">
              <button
                onClick={handleSave}
                disabled={saving || !editUrl.trim()}
                className="btn-primary btn-sm"
                style={{ opacity: (saving || !editUrl.trim()) ? 0.5 : 1 }}
              >
                {saving ? 'Saving...' : 'Save'}
              </button>
              <button onClick={handleCancel} className="btn-secondary btn-sm">
                Cancel
              </button>
            </div>
          </div>
        ) : schemaServiceUrl ? (
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

      {saveResult === 'success' && (
        <div className="card card-success p-3">
          <p className="text-xs text-gruvbox-green">
            Schema service URL updated. New connections will use the updated URL.
          </p>
        </div>
      )}

      <div className="card card-info p-3">
        <p className="text-xs text-secondary">
          <strong>Tip:</strong> Use <code className="text-gruvbox-blue">./run.sh --dev</code> to start with the dev schema service, or <code className="text-gruvbox-blue">./run.sh --local-schema</code> for fully offline development.
        </p>
      </div>
    </div>
  )
}

export default SchemaServiceSettings
