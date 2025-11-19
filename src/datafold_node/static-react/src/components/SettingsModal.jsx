import { useState, useEffect } from 'react'
import { ingestionClient } from '../api/clients'
import TransformsTab from './tabs/TransformsTab'
import KeyManagementTab from './tabs/KeyManagementTab'
import { useSchemaServiceConfig, SCHEMA_SERVICE_ENVIRONMENTS } from '../contexts/SchemaServiceConfigContext'
import { checkSchemaServiceStatus } from '../api/clients/configuredSchemaClient'

function SettingsModal({ isOpen, onClose }) {
  const [activeTab, setActiveTab] = useState('ai')
  const [aiProvider, setAiProvider] = useState('OpenRouter')
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [openrouterModel, setOpenrouterModel] = useState('anthropic/claude-3.5-sonnet')
  const [openrouterBaseUrl, setOpenrouterBaseUrl] = useState('https://openrouter.ai/api/v1')
  const [ollamaModel, setOllamaModel] = useState('llama3')
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState('http://localhost:11434')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)
  const [showAdvanced, setShowAdvanced] = useState(false)
  
  // Schema service configuration
  const { environment, setEnvironment } = useSchemaServiceConfig()
  const [selectedSchemaEnv, setSelectedSchemaEnv] = useState(environment.id)
  const [connectionStatus, setConnectionStatus] = useState({})
  const [checkingStatus, setCheckingStatus] = useState({})

  useEffect(() => {
    if (isOpen) {
      loadAiConfig()
      setSelectedSchemaEnv(environment.id)
      // Auto-check current environment status when opening the schema service tab
      if (activeTab === 'schema-service') {
        checkStatus(environment.id)
      }
    }
  }, [isOpen, environment.id, activeTab])

  const loadAiConfig = async () => {
    try {
      const response = await ingestionClient.getConfig()
      if (response.success) {
        setOpenrouterApiKey(response.data.openrouter.api_key || '')
        setOpenrouterModel(response.data.openrouter.model || 'anthropic/claude-3.5-sonnet')
        setOpenrouterBaseUrl(response.data.openrouter.base_url || 'https://openrouter.ai/api/v1')
        setOllamaModel(response.data.ollama.model || 'llama3')
        setOllamaBaseUrl(response.data.ollama.base_url || 'http://localhost:11434')
        setAiProvider(response.data.provider || 'OpenRouter')
      }
    } catch (error) {
      console.error('Failed to load AI config:', error)
    }
  }

  const saveAiConfig = async () => {
    try {
      const config = {
        provider: aiProvider,
        openrouter: {
          api_key: openrouterApiKey,
          model: openrouterModel,
          base_url: openrouterBaseUrl,
        },
        ollama: {
          model: ollamaModel,
          base_url: ollamaBaseUrl,
        },
      }

      const response = await ingestionClient.saveConfig(config)
      
      if (response.success) {
        setConfigSaveStatus({ success: true, message: 'Configuration saved successfully' })
        setTimeout(() => {
          setConfigSaveStatus(null)
          onClose()
        }, 1500)
      } else {
        setConfigSaveStatus({ success: false, message: 'Failed to save configuration' })
      }
    } catch (error) {
      setConfigSaveStatus({ success: false, message: error.message || 'Failed to save configuration' })
    }

    setTimeout(() => setConfigSaveStatus(null), 3000)
  }

  const checkStatus = async (envId) => {
    const env = Object.values(SCHEMA_SERVICE_ENVIRONMENTS).find(e => e.id === envId)
    if (!env) return
    
    setCheckingStatus(prev => ({ ...prev, [envId]: true }))
    
    try {
      const result = await checkSchemaServiceStatus(env.baseUrl)
      setConnectionStatus(prev => ({
        ...prev,
        [envId]: result
      }))
    } catch (error) {
      setConnectionStatus(prev => ({
        ...prev,
        [envId]: { success: false, error: error.message }
      }))
    } finally {
      setCheckingStatus(prev => ({ ...prev, [envId]: false }))
    }
  }

  const saveSchemaServiceConfig = () => {
    setEnvironment(selectedSchemaEnv)
    setConfigSaveStatus({ success: true, message: 'Schema service environment updated successfully' })
    setTimeout(() => {
      setConfigSaveStatus(null)
      onClose()
    }, 1500)
  }

  const getStatusBadge = (envId) => {
    const status = connectionStatus[envId]
    const checking = checkingStatus[envId]

    if (checking) {
      return (
        <span className="inline-flex items-center text-xs bg-gray-100 text-gray-700 px-2 py-1 rounded">
          <svg className="animate-spin h-3 w-3 mr-1" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
          Checking...
        </span>
      )
    }

    if (!status) {
      return (
        <button
          onClick={(e) => {
            e.stopPropagation()
            checkStatus(envId)
          }}
          className="text-xs text-blue-600 hover:text-blue-700 underline"
        >
          Test Connection
        </button>
      )
    }

    if (status.success) {
      return (
        <span className="inline-flex items-center text-xs bg-green-100 text-green-700 px-2 py-1 rounded">
          ✓ Online {status.responseTime && `(${status.responseTime}ms)`}
        </span>
      )
    }

    return (
      <span className="inline-flex items-center text-xs bg-red-100 text-red-700 px-2 py-1 rounded" title={status.error}>
        ✗ Offline
      </span>
    )
  }

  if (!isOpen) return null

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        {/* Background overlay */}
        <div
          className="fixed inset-0 transition-opacity bg-gray-500 bg-opacity-75"
          onClick={onClose}
        />

        {/* Modal panel */}
        <div className="inline-block align-bottom bg-white rounded-lg text-left overflow-hidden shadow-xl transform transition-all sm:my-8 sm:align-middle sm:max-w-4xl sm:w-full">
          <div className="bg-white">
            <div className="flex items-center justify-between px-6 pt-5 pb-4 border-b border-gray-200">
              <h3 className="text-lg font-medium text-gray-900">Settings</h3>
              <button
                onClick={onClose}
                className="text-gray-400 hover:text-gray-600 transition-colors"
              >
                <svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
                </svg>
              </button>
            </div>

            {/* Tabs */}
            <div className="border-b border-gray-200">
              <nav className="flex px-6">
                <button
                  onClick={() => setActiveTab('ai')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'ai'
                      ? 'border-blue-500 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  AI Configuration
                </button>
                <button
                  onClick={() => setActiveTab('transforms')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'transforms'
                      ? 'border-blue-500 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  Transforms
                </button>
                <button
                  onClick={() => setActiveTab('keys')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'keys'
                      ? 'border-blue-500 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  Key Management
                </button>
                <button
                  onClick={() => setActiveTab('schema-service')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'schema-service'
                      ? 'border-blue-500 text-blue-600'
                      : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                  }`}
                >
                  Schema Service
                </button>
              </nav>
            </div>

            <div className="px-6 py-4 max-h-[70vh] overflow-y-auto">
              {activeTab === 'ai' && (
                <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Provider
                  </label>
                  <select
                    value={aiProvider}
                    onChange={(e) => setAiProvider(e.target.value)}
                    className="w-full p-2 border border-gray-300 rounded text-sm"
                  >
                    <option value="OpenRouter">OpenRouter</option>
                    <option value="Ollama">Ollama</option>
                  </select>
                </div>

                {aiProvider === 'OpenRouter' ? (
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Model
                    </label>
                    <select
                      value={openrouterModel}
                      onChange={(e) => setOpenrouterModel(e.target.value)}
                      className="w-full p-2 border border-gray-300 rounded text-sm"
                    >
                      <option value="anthropic/claude-3.5-sonnet">Claude 3.5 Sonnet</option>
                      <option value="anthropic/claude-3.5-haiku">Claude 3.5 Haiku</option>
                      <option value="openai/gpt-4o">GPT-4o</option>
                      <option value="openai/gpt-4o-mini">GPT-4o Mini</option>
                      <option value="openai/o1">OpenAI o1</option>
                      <option value="openai/o1-mini">OpenAI o1-mini</option>
                      <option value="google/gemini-2.0-flash-exp">Gemini 2.0 Flash</option>
                      <option value="google/gemini-pro-1.5">Gemini 1.5 Pro</option>
                      <option value="meta-llama/llama-3.3-70b-instruct">Llama 3.3 70B</option>
                      <option value="meta-llama/llama-3.1-405b-instruct">Llama 3.1 405B</option>
                      <option value="deepseek/deepseek-chat">DeepSeek Chat</option>
                      <option value="qwen/qwen-2.5-72b-instruct">Qwen 2.5 72B</option>
                    </select>
                  </div>
                ) : (
                  <div>
                    <label className="block text-sm font-medium text-gray-700 mb-1">
                      Model
                    </label>
                    <input
                      type="text"
                      value={ollamaModel}
                      onChange={(e) => setOllamaModel(e.target.value)}
                      placeholder="e.g., llama3"
                      className="w-full p-2 border border-gray-300 rounded text-sm"
                    />
                  </div>
                )}
              </div>

              {aiProvider === 'OpenRouter' && (
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    API Key <span className="text-xs text-gray-500">(<a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline">get key</a>)</span>
                  </label>
                  <input
                    type="password"
                    value={openrouterApiKey}
                    onChange={(e) => setOpenrouterApiKey(e.target.value)}
                    placeholder="sk-or-..."
                    className="w-full p-2 border border-gray-300 rounded text-sm"
                  />
                </div>
              )}

              {/* Advanced Settings */}
              <div>
                <button
                  onClick={() => setShowAdvanced(!showAdvanced)}
                  className="text-sm text-gray-600 hover:text-gray-800 flex items-center gap-1"
                >
                  <span>{showAdvanced ? '▼' : '▶'}</span>
                  Advanced Settings
                </button>
                
                {showAdvanced && (
                  <div className="mt-3 space-y-3 pl-4 border-l-2 border-gray-200">
                    <div>
                      <label className="block text-sm font-medium text-gray-700 mb-1">
                        Base URL
                      </label>
                      <input
                        type="text"
                        value={aiProvider === 'OpenRouter' ? openrouterBaseUrl : ollamaBaseUrl}
                        onChange={(e) => aiProvider === 'OpenRouter' 
                          ? setOpenrouterBaseUrl(e.target.value)
                          : setOllamaBaseUrl(e.target.value)
                        }
                        placeholder={aiProvider === 'OpenRouter' 
                          ? 'https://openrouter.ai/api/v1'
                          : 'http://localhost:11434'
                        }
                        className="w-full p-2 border border-gray-300 rounded text-sm"
                      />
                    </div>
                  </div>
                )}
              </div>

                  {configSaveStatus && (
                    <div className={`p-3 rounded-md ${
                      configSaveStatus.success 
                        ? 'bg-green-50 text-green-800 border border-green-200' 
                        : 'bg-red-50 text-red-800 border border-red-200'
                    }`}>
                      <span className="text-sm font-medium">
                        {configSaveStatus.success ? '✓' : '✗'} {configSaveStatus.message}
                      </span>
                    </div>
                  )}
                </div>
              )}

              {activeTab === 'transforms' && (
                <TransformsTab onResult={() => {}} />
              )}

              {activeTab === 'keys' && (
                <KeyManagementTab onResult={() => {}} />
              )}

              {activeTab === 'schema-service' && (
                <div className="space-y-4">
                  <div className="mb-4">
                    <h4 className="text-md font-semibold text-gray-900 mb-2">Schema Service Environment</h4>
                    <p className="text-sm text-gray-600 mb-4">
                      Select which schema service endpoint to use. This affects where schemas are loaded from and saved to.
                    </p>
                  </div>

                  <div className="space-y-3">
                    {Object.values(SCHEMA_SERVICE_ENVIRONMENTS).map(env => (
                      <label
                        key={env.id}
                        className={`flex items-start p-4 border-2 rounded-lg cursor-pointer transition-all ${
                          selectedSchemaEnv === env.id
                            ? 'border-blue-500 bg-blue-50'
                            : 'border-gray-200 hover:border-gray-300 bg-white'
                        }`}
                      >
                        <input
                          type="radio"
                          name="schemaEnvironment"
                          value={env.id}
                          checked={selectedSchemaEnv === env.id}
                          onChange={(e) => setSelectedSchemaEnv(e.target.value)}
                          className="mt-1 mr-3"
                        />
                        <div className="flex-1">
                          <div className="flex items-center justify-between mb-2">
                            <span className="text-sm font-semibold text-gray-900">{env.name}</span>
                            <div className="flex items-center gap-2">
                              {getStatusBadge(env.id)}
                              {selectedSchemaEnv === env.id && (
                                <span className="text-xs bg-blue-100 text-blue-700 px-2 py-1 rounded">Active</span>
                              )}
                            </div>
                          </div>
                          <p className="text-xs text-gray-600 mt-1">{env.description}</p>
                          <p className="text-xs text-gray-500 mt-1 font-mono">
                            {env.baseUrl || window.location.origin}
                          </p>
                          {connectionStatus[env.id] && !connectionStatus[env.id].success && (
                            <p className="text-xs text-red-600 mt-1">
                              Error: {connectionStatus[env.id].error}
                            </p>
                          )}
                        </div>
                      </label>
                    ))}
                  </div>

                  {configSaveStatus && (
                    <div className={`p-3 rounded-md ${
                      configSaveStatus.success 
                        ? 'bg-green-50 text-green-800 border border-green-200' 
                        : 'bg-red-50 text-red-800 border border-red-200'
                    }`}>
                      <span className="text-sm font-medium">
                        {configSaveStatus.success ? '✓' : '✗'} {configSaveStatus.message}
                      </span>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>

          <div className="bg-gray-50 px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-gray-200">
            {activeTab === 'ai' || activeTab === 'schema-service' ? (
              <>
                <button
                  onClick={activeTab === 'ai' ? saveAiConfig : saveSchemaServiceConfig}
                  className="w-full inline-flex justify-center rounded-md border border-transparent shadow-sm px-4 py-2 bg-blue-600 text-base font-medium text-white hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:ml-3 sm:w-auto sm:text-sm"
                >
                  Save Configuration
                </button>
                <button
                  onClick={onClose}
                  className="mt-3 w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:mt-0 sm:w-auto sm:text-sm"
                >
                  Cancel
                </button>
              </>
            ) : (
              <button
                onClick={onClose}
                className="w-full inline-flex justify-center rounded-md border border-gray-300 shadow-sm px-4 py-2 bg-white text-base font-medium text-gray-700 hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-blue-500 sm:w-auto sm:text-sm"
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

