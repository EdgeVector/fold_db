import { useState, useEffect } from 'react'
import TransformsTab from './tabs/TransformsTab'
import KeyManagementTab from './tabs/KeyManagementTab'
import AiConfigSettings from './settings/AiConfigSettings'
import SchemaServiceSettings from './settings/SchemaServiceSettings'
import DatabaseSettings from './settings/DatabaseSettings'

function SettingsModal({ isOpen, onClose, onConfigSaved, initialTab }) {
  const [activeTab, setActiveTab] = useState(initialTab || 'ai')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)

  useEffect(() => {
    if (isOpen && initialTab) setActiveTab(initialTab)
  }, [isOpen, initialTab])

  const aiConfig = AiConfigSettings({ configSaveStatus, setConfigSaveStatus, onClose, onConfigSaved })
  const dbConfig = DatabaseSettings({ configSaveStatus, setConfigSaveStatus, onClose })

  if (!isOpen) return null

  const tabs = [
    { id: 'ai', label: 'AI Configuration' },
    { id: 'transforms', label: 'Transforms' },
    { id: 'keys', label: 'Key Management' },
    { id: 'schema-service', label: 'Schema Service' },
    { id: 'database', label: 'Database' },
  ]

  const handleSave = () => {
    if (activeTab === 'ai') aiConfig.saveAiConfig()
    else if (activeTab === 'database') dbConfig.saveDatabaseConfig()
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3 className="text-lg font-medium">Settings</h3>
          <button onClick={onClose} className="btn-secondary btn-sm p-1">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="flex border-b border-border px-6">
          {tabs.map(t => (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id)}
              className={`tab ${activeTab === t.id ? 'tab-active' : ''}`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="modal-body">
          {activeTab === 'ai' && aiConfig.content}
          {activeTab === 'transforms' && <TransformsTab onResult={() => {}} />}
          {activeTab === 'keys' && <KeyManagementTab onResult={() => {}} />}
          {activeTab === 'schema-service' && <SchemaServiceSettings />}
          {activeTab === 'database' && dbConfig.content}
        </div>

        <div className="modal-footer">
          {activeTab === 'ai' || activeTab === 'database' ? (
            <>
              <button onClick={onClose} className="btn-secondary">Cancel</button>
              <button onClick={handleSave} className="btn-primary">
                {activeTab === 'database' ? 'Save and Restart DB' : 'Save Configuration'}
              </button>
            </>
          ) : (
            <button onClick={onClose} className="btn-secondary">Close</button>
          )}
        </div>
      </div>
    </div>
  )
}

export default SettingsModal
