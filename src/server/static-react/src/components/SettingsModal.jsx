import { useState, useEffect } from 'react'
import { ingestionClient } from '../api/clients'
import { getDatabaseConfig, updateDatabaseConfig, resetDatabase, getSystemStatus } from '../api/clients/systemClient'
import { TrashIcon } from '@heroicons/react/24/solid'
import TransformsTab from './tabs/TransformsTab'
import KeyManagementTab from './tabs/KeyManagementTab'

function SettingsModal({ isOpen, onClose }) {
  const [activeTab, setActiveTab] = useState('ai')
  const [aiProvider, setAiProvider] = useState('OpenRouter')
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [openrouterModel, setOpenrouterModel] = useState('anthropic/claude-3.5-sonnet')
  const [openrouterBaseUrl, setOpenrouterBaseUrl] = useState('https://openrouter.ai/api/v1')
  const [ollamaModel, setOllamaModel] = useState('llama3')
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState('http://192.168.1.226:11434')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)
  const [showAdvanced, setShowAdvanced] = useState(false)
  
  // Schema service status (from backend)
  const [schemaServiceUrl, setSchemaServiceUrl] = useState(null)
  const [schemaServiceLoading, setSchemaServiceLoading] = useState(false)
  
  // Database configuration
  const [dbType, setDbType] = useState('local')
  const [dbPath, setDbPath] = useState('data')
  const [dynamoTableName, setDynamoTableName] = useState('DataFoldStorage')
  const [dynamoRegion, setDynamoRegion] = useState('us-west-2')
  const [dynamoUserId, setDynamoUserId] = useState('')
  const [s3Bucket, setS3Bucket] = useState('')
  const [s3Region, setS3Region] = useState('us-east-1')
  const [s3Prefix, setS3Prefix] = useState('folddb')
  const [s3LocalPath, setS3LocalPath] = useState('/tmp/folddb-data')
  
  // Database reset state
  const [showResetConfirm, setShowResetConfirm] = useState(false)
  const [isResetting, setIsResetting] = useState(false)
  const [resetResult, setResetResult] = useState(null)

  useEffect(() => {
    if (isOpen) {
      loadAiConfig()
      loadDatabaseConfig()
      // Load schema service URL when opening the schema service tab
      if (activeTab === 'schema-service') {
        loadSchemaServiceStatus()
      }
    }
  }, [isOpen, activeTab])

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

  const loadAiConfig = async () => {
    try {
      const response = await ingestionClient.getConfig()
      if (response.success) {
        setOpenrouterApiKey(response.data.openrouter.api_key || '')
        setOpenrouterModel(response.data.openrouter.model || 'anthropic/claude-3.5-sonnet')
        setOpenrouterBaseUrl(response.data.openrouter.base_url || 'https://openrouter.ai/api/v1')
        setOllamaModel(response.data.ollama.model || 'llama3')
        setOllamaBaseUrl(response.data.ollama.base_url || 'http://192.168.1.226:11434')
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


  const loadDatabaseConfig = async () => {
    try {
      const response = await getDatabaseConfig()
      if (response.success && response.data) {
        const config = response.data
        setDbType(config.type)
        if (config.type === 'local') {
          setDbPath(config.path || 'data')
        } else if (config.type === 'dynamodb') {
          setDynamoTableName(config.table_name || 'DataFoldStorage')
          setDynamoRegion(config.region || 'us-west-2')
          setDynamoUserId(config.user_id || '')
        } else if (config.type === 's3') {
          setS3Bucket(config.bucket || '')
          setS3Region(config.region || 'us-east-1')
          setS3Prefix(config.prefix || 'folddb')
          setS3LocalPath(config.local_path || '/tmp/folddb-data')
        }
      }
    } catch (error) {
      console.error('Failed to load database config:', error)
    }
  }

  const saveDatabaseConfig = async () => {
    try {
      let config
      if (dbType === 'local') {
        config = {
          type: 'local',
          path: dbPath
        }
      } else if (dbType === 'dynamodb') {
        if (!dynamoTableName || !dynamoRegion) {
          setConfigSaveStatus({ success: false, message: 'Table name and region are required for DynamoDB' })
          setTimeout(() => setConfigSaveStatus(null), 3000)
          return
        }
        config = {
          type: 'dynamodb',
          table_name: dynamoTableName,
          region: dynamoRegion,
          user_id: dynamoUserId || undefined
        }
      } else if (dbType === 's3') {
        if (!s3Bucket || !s3Region) {
          setConfigSaveStatus({ success: false, message: 'Bucket and region are required for S3' })
          setTimeout(() => setConfigSaveStatus(null), 3000)
          return
        }
        config = {
          type: 's3',
          bucket: s3Bucket,
          region: s3Region,
          prefix: s3Prefix || 'folddb',
          local_path: s3LocalPath || '/tmp/folddb-data'
        }
      }

      const response = await updateDatabaseConfig(config)
      
      if (response.success) {
        setConfigSaveStatus({ 
          success: true, 
          message: response.data.requires_restart 
            ? 'Database configuration saved. Please restart the server for changes to take effect.'
            : response.data.message || 'Database configuration saved and restarted successfully'
        })
        setTimeout(() => {
          setConfigSaveStatus(null)
          if (!response.data.requires_restart) {
            onClose()
          }
        }, 3000)
      } else {
        setConfigSaveStatus({ success: false, message: response.error || 'Failed to save database configuration' })
      }
    } catch (error) {
      setConfigSaveStatus({ success: false, message: error.message || 'Failed to save database configuration' })
    }
    setTimeout(() => setConfigSaveStatus(null), 5000)
  }


  const handleResetDatabase = async () => {
    setIsResetting(true)
    setResetResult(null)

    try {
      const response = await resetDatabase(true)

      if (response.success && response.data) {
        if (response.data.job_id) {
          setResetResult({ 
            type: 'success', 
            message: `Reset started (Job: ${response.data.job_id.substring(0, 8)}...). The database will be reset in the background.`
          })
          setShowResetConfirm(false)
          setIsResetting(false)
        } else {
          setResetResult({ type: 'success', message: response.data.message || 'Database reset successfully' })
          setTimeout(() => {
            window.location.reload()
          }, 2000)
        }
      } else {
        setResetResult({ type: 'error', message: response.error || 'Reset failed' })
        setShowResetConfirm(false)
        setIsResetting(false)
      }
    } catch (error) {
      setResetResult({ type: 'error', message: `Network error: ${error.message}` })
      setShowResetConfirm(false)
      setIsResetting(false)
    }
  }


  if (!isOpen) return null

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
                <button
                  onClick={() => setActiveTab('ai')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'ai'
                      ? 'border-terminal-green text-terminal-green'
                      : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                  }`}
                >
                  AI Configuration
                </button>
                <button
                  onClick={() => setActiveTab('transforms')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'transforms'
                      ? 'border-terminal-green text-terminal-green'
                      : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                  }`}
                >
                  Transforms
                </button>
                <button
                  onClick={() => setActiveTab('keys')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'keys'
                      ? 'border-terminal-green text-terminal-green'
                      : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                  }`}
                >
                  Key Management
                </button>
                <button
                  onClick={() => setActiveTab('schema-service')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'schema-service'
                      ? 'border-terminal-green text-terminal-green'
                      : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                  }`}
                >
                  Schema Service
                </button>
                <button
                  onClick={() => setActiveTab('database')}
                  className={`py-3 px-4 text-sm font-medium border-b-2 transition-colors ${
                    activeTab === 'database'
                      ? 'border-terminal-green text-terminal-green'
                      : 'border-transparent text-terminal-dim hover:text-terminal hover:border-terminal'
                  }`}
                >
                  Database
                </button>
              </nav>
            </div>

            <div className="px-6 py-4 max-h-[70vh] overflow-y-auto">
              {activeTab === 'ai' && (
                <div className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <label className="block text-sm font-medium text-terminal-dim mb-1">
                    --provider
                  </label>
                  <select
                    value={aiProvider}
                    onChange={(e) => setAiProvider(e.target.value)}
                    className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                  >
                    <option value="OpenRouter">OpenRouter</option>
                    <option value="Ollama">Ollama</option>
                  </select>
                </div>

                {aiProvider === 'OpenRouter' ? (
                  <div>
                    <label className="block text-sm font-medium text-terminal-dim mb-1">
                      --model
                    </label>
                    <select
                      value={openrouterModel}
                      onChange={(e) => setOpenrouterModel(e.target.value)}
                      className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
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
                    <label className="block text-sm font-medium text-terminal-dim mb-1">
                      --model
                    </label>
                    <select
                      value={ollamaModel}
                      onChange={(e) => setOllamaModel(e.target.value)}
                      className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                    >
                      <option value="llama3.3">Llama 3.3 (70B)</option>
                      <option value="llama3.2">Llama 3.2 (3B)</option>
                      <option value="llama3.1">Llama 3.1 (8B)</option>
                      <option value="llama3">Llama 3 (8B)</option>
                      <option value="mistral">Mistral (7B)</option>
                      <option value="mixtral">Mixtral 8x7B</option>
                      <option value="codellama">Code Llama (7B)</option>
                      <option value="deepseek-coder-v2">DeepSeek Coder V2</option>
                      <option value="qwen2.5">Qwen 2.5 (7B)</option>
                      <option value="phi3">Phi-3 (3.8B)</option>
                      <option value="gemma2">Gemma 2 (9B)</option>
                    </select>
                    <p className="text-xs text-terminal-dim mt-1">
                      Requires Ollama running locally. Pull model with: <code className="text-terminal-cyan">ollama pull {ollamaModel}</code>
                    </p>
                  </div>
                )}
              </div>

              {aiProvider === 'OpenRouter' && (
                <div>
                  <label className="block text-sm font-medium text-terminal-dim mb-1">
                    --api-key <span className="text-xs text-terminal-dim">(<a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-terminal-cyan hover:underline">get key</a>)</span>
                  </label>
                  <input
                    type="password"
                    value={openrouterApiKey}
                    onChange={(e) => setOpenrouterApiKey(e.target.value)}
                    placeholder="sk-or-..."
                    className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                  />
                </div>
              )}

              {/* Advanced Settings */}
              <div>
                <button
                  onClick={() => setShowAdvanced(!showAdvanced)}
                  className="text-sm text-terminal-dim hover:text-terminal flex items-center gap-1"
                >
                  <span>{showAdvanced ? '▼' : '▶'}</span>
                  --advanced
                </button>
                
                {showAdvanced && (
                  <div className="mt-3 space-y-3 pl-4 border-l-2 border-terminal">
                    <div>
                      <label className="block text-sm font-medium text-terminal-dim mb-1">
                        --base-url
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
                          : 'http://192.168.1.226:11434'
                        }
                        className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                      />
                    </div>
                  </div>
                )}
              </div>

                  {configSaveStatus && (
                    <div className={`p-3 border-l-4 ${
                      configSaveStatus.success 
                        ? 'border-terminal-green text-terminal-green' 
                        : 'border-terminal-red text-terminal-red'
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
              )}

              {activeTab === 'database' && (
                <div className="space-y-4">
                  <div className="mb-4">
                    <h4 className="text-md font-semibold text-terminal-green mb-2"># Database Storage Backend</h4>
                    <p className="text-sm text-terminal-dim mb-4">
                      Choose the storage backend for your database. Changes require a server restart.
                    </p>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-terminal-dim mb-2">
                      --storage-type
                    </label>
                    <select
                      value={dbType}
                      onChange={(e) => setDbType(e.target.value)}
                      className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                    >
                      <option value="local">Local (Sled)</option>
                      <option value="dynamodb">DynamoDB</option>
                      <option value="s3">S3</option>
                    </select>
                  </div>

                  {dbType === 'local' ? (
                    <div>
                      <label className="block text-sm font-medium text-terminal-dim mb-1">
                        --path
                      </label>
                      <input
                        type="text"
                        value={dbPath}
                        onChange={(e) => setDbPath(e.target.value)}
                        placeholder="data"
                        className="w-full p-2 border border-terminal bg-terminal text-terminal text-sm"
                      />
                      <p className="text-xs text-terminal-dim mt-1">
                        Local filesystem path where the database will be stored
                      </p>
                    </div>
                  ) : dbType === 'dynamodb' ? (
                    <div className="space-y-3">
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          Table Name <span className="text-red-500">*</span>
                        </label>
                        <input
                          type="text"
                          value={dynamoTableName}
                          onChange={(e) => setDynamoTableName(e.target.value)}
                          placeholder="DataFoldStorage"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          Base table name (namespaces will be appended automatically)
                        </p>
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          AWS Region <span className="text-red-500">*</span>
                        </label>
                        <input
                          type="text"
                          value={dynamoRegion}
                          onChange={(e) => setDynamoRegion(e.target.value)}
                          placeholder="us-west-2"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          AWS region where your DynamoDB tables are located
                        </p>
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          User ID (Optional)
                        </label>
                        <input
                          type="text"
                          value={dynamoUserId}
                          onChange={(e) => setDynamoUserId(e.target.value)}
                          placeholder="Leave empty for single-tenant"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          User ID for multi-tenant isolation (uses partition key)
                        </p>
                      </div>
                      <div className="p-3 card-terminal border border-terminal-yellow ">
                        <p className="text-xs text-terminal-yellow">
                          <strong>Note:</strong> Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). 
                          The DynamoDB tables will be created automatically if they don't exist.
                        </p>
                      </div>
                    </div>
                  ) : (
                    <div className="space-y-3">
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          S3 Bucket <span className="text-red-500">*</span>
                        </label>
                        <input
                          type="text"
                          value={s3Bucket}
                          onChange={(e) => setS3Bucket(e.target.value)}
                          placeholder="my-datafold-bucket"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          S3 bucket name where the database will be stored
                        </p>
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          AWS Region <span className="text-red-500">*</span>
                        </label>
                        <input
                          type="text"
                          value={s3Region}
                          onChange={(e) => setS3Region(e.target.value)}
                          placeholder="us-east-1"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          AWS region where your S3 bucket is located
                        </p>
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          S3 Prefix (Optional)
                        </label>
                        <input
                          type="text"
                          value={s3Prefix}
                          onChange={(e) => setS3Prefix(e.target.value)}
                          placeholder="folddb"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          Prefix/path within the bucket (defaults to "folddb")
                        </p>
                      </div>
                      <div>
                        <label className="block text-sm font-medium text-terminal-dim mb-1">
                          Local Cache Path
                        </label>
                        <input
                          type="text"
                          value={s3LocalPath}
                          onChange={(e) => setS3LocalPath(e.target.value)}
                          placeholder="/tmp/folddb-data"
                          className="w-full p-2 border border-terminal text-sm"
                        />
                        <p className="text-xs text-terminal-dim mt-1">
                          Local filesystem path for caching S3 data (defaults to /tmp/folddb-data)
                        </p>
                      </div>
                      <div className="p-3 card-terminal border border-terminal-yellow ">
                        <p className="text-xs text-terminal-yellow">
                          <strong>Note:</strong> Ensure your AWS credentials are configured (via environment variables, IAM role, or AWS CLI). 
                          The database will be synced to/from S3 on startup and shutdown.
                        </p>
                      </div>
                    </div>
                  )}

                  {/* Danger Zone - Reset Database */}
                  <div className="mt-8 pt-6 border-t border-terminal-red">
                    <div className="flex items-center gap-2 mb-3">
                      <TrashIcon className="w-5 h-5 text-red-500" />
                      <h4 className="text-md font-semibold text-terminal-red">Danger Zone</h4>
                    </div>
                    <p className="text-sm text-terminal-dim mb-4">
                      Permanently delete all data and restart the database. This action cannot be undone.
                    </p>
                    
                    {!showResetConfirm ? (
                      <button
                        onClick={() => setShowResetConfirm(true)}
                        className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-red-600 border border-red-300  hover:card-terminal hover:border-red-400 transition-colors"
                      >
                        <TrashIcon className="w-4 h-4" />
                        Reset Database
                      </button>
                    ) : (
                      <div className="p-4 card-terminal border border-terminal-red ">
                        <p className="text-sm text-terminal-red mb-3">
                          <strong>Are you sure?</strong> This will:
                        </p>
                        <ul className="list-disc list-inside text-sm text-red-700 mb-4 space-y-1">
                          <li>Remove all schemas</li>
                          <li>Delete all stored data</li>
                          <li>Reset network connections</li>
                        </ul>
                        <div className="flex gap-3">
                          <button
                            onClick={handleResetDatabase}
                            disabled={isResetting}
                            className="px-4 py-2 text-sm font-medium text-white bg-red-600  hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                          >
                            {isResetting ? 'Resetting...' : 'Yes, Reset Database'}
                          </button>
                          <button
                            onClick={() => setShowResetConfirm(false)}
                            disabled={isResetting}
                            className="px-4 py-2 text-sm font-medium text-terminal-dim bg-terminal border border-terminal  hover:bg-terminal transition-colors"
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    )}

                    {resetResult && (
                      <div className={`mt-4 p-3  text-sm ${resetResult.type === 'success'
                          ? 'text-terminal-green border border-terminal-green'
                          : 'card-terminal text-terminal-red border border-terminal-red'
                        }`}>
                        {resetResult.message}
                      </div>
                    )}
                  </div>

                  {configSaveStatus && (
                    <div className={`p-3  ${
                      configSaveStatus.success 
                        ? 'text-terminal-green border border-terminal-green' 
                        : 'card-terminal text-terminal-red border border-terminal-red'
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

          <div className="bg-terminal px-4 py-3 sm:px-6 sm:flex sm:flex-row-reverse gap-3 border-t border-terminal">
            {activeTab === 'ai' || activeTab === 'database' ? (
              <>
                <button
                  onClick={activeTab === 'ai' ? saveAiConfig : saveDatabaseConfig}
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

