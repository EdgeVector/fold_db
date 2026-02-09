import { useState } from 'react'
import TransformsTab from './tabs/TransformsTab'
import KeyManagementTab from './tabs/KeyManagementTab'
import AiConfigSettings from './settings/AiConfigSettings'
import SchemaServiceSettings from './settings/SchemaServiceSettings'
import DatabaseSettings from './settings/DatabaseSettings'

function SettingsModal({ isOpen, onClose }) {
  const [activeTab, setActiveTab] = useState('ai')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)

  // Sub-component instances (using render-prop pattern for save handlers)
  const aiConfig = AiConfigSettings({ configSaveStatus, setConfigSaveStatus, onClose })
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
    if (activeTab === 'ai') {
      aiConfig.saveAiConfig()
    } else if (activeTab === 'database') {
      dbConfig.saveDatabaseConfig()
    }
  }

  return (
    <div className="minimal-modal-overlay" onClick={onClose}>
      <div className="minimal-modal" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="minimal-modal-header">
          <h3>Settings</h3>
          <button onClick={onClose} className="minimal-modal-close">
            <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        {/* Tabs */}
        <div className="minimal-modal-tabs">
          {tabs.map(tab => (
            <button
              key={tab.id}
              onClick={() => setActiveTab(tab.id)}
              className={`minimal-modal-tab ${activeTab === tab.id ? 'active' : ''}`}
            >
              {tab.label}
            </button>
          ))}
        </div>

        {/* Body */}
        <div className="minimal-modal-body">
          {activeTab === 'ai' && aiConfig.content}
          {activeTab === 'transforms' && <TransformsTab onResult={() => {}} />}
          {activeTab === 'keys' && <KeyManagementTab onResult={() => {}} />}
          {activeTab === 'schema-service' && <SchemaServiceSettings />}
          {activeTab === 'database' && dbConfig.content}
        </div>

        {/* Footer */}
        <div className="minimal-modal-footer">
          {activeTab === 'ai' || activeTab === 'database' ? (
            <>
              <button onClick={onClose} className="minimal-btn-secondary text-sm">
                Cancel
              </button>
              <button onClick={handleSave} className="minimal-btn text-sm">
                {activeTab === 'database' ? 'Save and Restart DB' : 'Save Configuration'}
              </button>
            </>
          ) : (
            <button onClick={onClose} className="minimal-btn-secondary text-sm">
              Close
            </button>
          )}
        </div>
      </div>
    </div>
  )
}

export default SettingsModal
