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
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        {/* Background overlay */}
        <div
          className="fixed inset-0 transition-opacity bg-black bg-opacity-80"
          onClick={onClose}
        />

        {/* Modal panel */}
        <div className="inline-block align-bottom card-terminal text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full border border-terminal">
          <div className="bg-terminal">
            <div className="flex items-center justify-between px-6 pt-5 pb-4 border-b border-terminal">
              <h3 className="text-lg font-medium text-terminal-green"><span className="text-terminal-dim">$</span> settings</h3>
              <button
                onClick={onClose}
                className="text-terminal-dim hover:text-terminal transition-colors"
              >
                <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Tabs */}
            <div className="border-b border-terminal">
              <nav className="flex px-6">
                {tabs.map(tab => (
                  <button
                    key={tab.id}
                    onClick={() => setActiveTab(tab.id)}
                    className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                      activeTab === tab.id
                        ? 'border-terminal-green text-terminal-green'
                        : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                    }`}
                  >
                    {tab.label}
                  </button>
                ))}
              </nav>
            </div>

            <div className="px-6 py-4 max-h-[70vh] overflow-y-auto">
              {activeTab === 'ai' && aiConfig.content}
              {activeTab === 'transforms' && <TransformsTab onResult={() => {}} />}
              {activeTab === 'keys' && <KeyManagementTab onResult={() => {}} />}
              {activeTab === 'schema-service' && <SchemaServiceSettings />}
              {activeTab === 'database' && dbConfig.content}
            </div>
          </div>

          <div className="bg-terminal px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-terminal">
            {activeTab === 'ai' || activeTab === 'database' ? (
              <>
                <button
                  onClick={handleSave}
                  className="btn-terminal btn-terminal-primary sm:ml-3 sm:w-auto sm:text-sm"
                >
                  → {activeTab === 'database' ? 'Save and Restart DB' : 'Save Configuration'}
                </button>
                <button
                  onClick={onClose}
                  className="btn-terminal sm:mt-0 sm:w-auto sm:text-sm"
                >
                  Cancel
                </button>
              </>
            ) : (
              <button
                onClick={onClose}
                className="btn-terminal sm:w-auto sm:text-sm"
              >
                Close
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

export default SettingsModal
