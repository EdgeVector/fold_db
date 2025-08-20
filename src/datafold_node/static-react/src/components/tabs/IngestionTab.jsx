import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'

function IngestionTab({ onResult }) {
  const [jsonData, setJsonData] = useState('')
  const [autoExecute, setAutoExecute] = useState(true)
  const [trustDistance, setTrustDistance] = useState(0)
  const [pubKey, setPubKey] = useState('default')
  const [isLoading, setIsLoading] = useState(false)
  const [ingestionStatus, setIngestionStatus] = useState(null)
  const [validationResult, setValidationResult] = useState(null)
  
  // OpenRouter configuration
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [openrouterModel, setOpenrouterModel] = useState('anthropic/claude-3.5-sonnet')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)

  useEffect(() => {
    fetchIngestionStatus()
    loadOpenRouterConfig()
  }, [])

  const fetchIngestionStatus = async () => {
    try {
      const response = await ingestionClient.getStatus()
      if (response.success) {
        setIngestionStatus(response.data)
      }
    } catch (error) {
      console.error('Failed to fetch ingestion status:', error)
    }
  }

  const loadOpenRouterConfig = async () => {
    try {
      const response = await ingestionClient.getOpenRouterConfig()
      if (response.success) {
        setOpenrouterApiKey(response.data.api_key || '')
        setOpenrouterModel(response.data.model || 'anthropic/claude-3.5-sonnet')
      }
    } catch (error) {
      console.error('Failed to load OpenRouter config:', error)
    }
  }

  const saveOpenRouterConfig = async () => {
    try {
      const config = {
        api_key: openrouterApiKey,
        model: openrouterModel
      }

      const response = await ingestionClient.saveConfig(config)
      
      if (response.success) {
        setConfigSaveStatus({ success: true, message: response.data.message || 'Configuration saved successfully' })
        // Refresh ingestion status to show updated config
        fetchIngestionStatus()
      } else {
        setConfigSaveStatus({ success: false, message: 'Failed to save configuration' })
      }
    } catch (error) {
      setConfigSaveStatus({ success: false, message: error.message || 'Failed to save configuration' })
    }

    // Clear status after 3 seconds
    setTimeout(() => setConfigSaveStatus(null), 3000)
  }

  const validateJson = async () => {
    if (!jsonData.trim()) {
      setValidationResult({ valid: false, error: 'JSON data is required' })
      return
    }

    try {
      const parsedData = JSON.parse(jsonData)
      const response = await ingestionClient.validateData(parsedData)
      
      if (response.success) {
        setValidationResult(response.data)
      } else {
        setValidationResult({
          valid: false,
          error: 'Validation failed'
        })
      }
    } catch (error) {
      setValidationResult({
        valid: false,
        error: error.message || 'Invalid JSON format'
      })
    }
  }

  const processIngestion = async () => {
    if (!jsonData.trim()) {
      onResult({
        success: false,
        error: 'JSON data is required'
      })
      return
    }

    setIsLoading(true)
    try {
      const parsedData = JSON.parse(jsonData)
      
      const options = {
        autoExecute,
        trustDistance,
        pubKey
      }

      const response = await ingestionClient.processIngestion(parsedData, options)
      
      if (response.success) {
        onResult(response.data)
        setJsonData('') // Clear the form on success
        setValidationResult(null)
      } else {
        onResult({
          success: false,
          error: 'Failed to process ingestion'
        })
      }
    } catch (error) {
      onResult({
        success: false,
        error: error.message || 'Failed to process ingestion'
      })
    } finally {
      setIsLoading(false)
    }
  }

  const loadSampleData = (sampleType) => {
    const samples = {
      user: {
        name: "John Doe",
        email: "john@example.com",
        age: 30,
        preferences: {
          theme: "dark",
          notifications: true
        }
      },
      product: {
        product_id: "LAPTOP001",
        name: "Gaming Laptop",
        price: 1299.99,
        category: "Electronics",
        specs: {
          cpu: "Intel i7",
          ram: "16GB",
          storage: "512GB SSD"
        },
        tags: ["gaming", "laptop", "high-performance"]
      },
      analytics: {
        event: "page_view",
        timestamp: "2024-01-15T10:30:00Z",
        user_id: "user123",
        page: "/products/laptop",
        metadata: {
          referrer: "google.com",
          user_agent: "Mozilla/5.0...",
          session_id: "sess_abc123"
        }
      }
    }
    
    setJsonData(JSON.stringify(samples[sampleType], null, 2))
    setValidationResult(null)
  }

  return (
    <div className="space-y-6">
      {/* Status Section */}
      {ingestionStatus && (
        <div className="bg-white p-4 rounded-lg shadow">
          <h3 className="text-lg font-medium text-gray-900 mb-3">Ingestion Service Status</h3>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
            <div>
              <span className="font-medium">Status:</span>
              <span className={`ml-2 px-2 py-1 rounded text-xs ${
                ingestionStatus.enabled && ingestionStatus.configured 
                  ? 'bg-green-100 text-green-800' 
                  : 'bg-red-100 text-red-800'
              }`}>
                {ingestionStatus.enabled && ingestionStatus.configured ? 'Ready' : 'Not Ready'}
              </span>
            </div>
            <div>
              <span className="font-medium">Model:</span>
              <span className="ml-2 text-gray-600">{ingestionStatus.model}</span>
            </div>
            <div>
              <span className="font-medium">Auto Execute:</span>
              <span className="ml-2 text-gray-600">{ingestionStatus.auto_execute_mutations ? 'Yes' : 'No'}</span>
            </div>
            <div>
              <span className="font-medium">Trust Distance:</span>
              <span className="ml-2 text-gray-600">{ingestionStatus.default_trust_distance}</span>
            </div>
          </div>
        </div>
      )}

      {/* Sample Data Section */}
      <div className="bg-white p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-3">Sample Data</h3>
        <div className="flex flex-wrap gap-2">
          <button
            onClick={() => loadSampleData('user')}
            className="px-3 py-1 bg-blue-100 text-blue-800 rounded text-sm hover:bg-blue-200"
          >
            User Profile
          </button>
          <button
            onClick={() => loadSampleData('product')}
            className="px-3 py-1 bg-green-100 text-green-800 rounded text-sm hover:bg-green-200"
          >
            Product Catalog
          </button>
          <button
            onClick={() => loadSampleData('analytics')}
            className="px-3 py-1 bg-purple-100 text-purple-800 rounded text-sm hover:bg-purple-200"
          >
            Analytics Event
          </button>
        </div>
      </div>

      {/* JSON Input Section */}
      <div className="bg-white p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-3">JSON Data Input</h3>
        
        <div className="space-y-4">
          <div>
            <label htmlFor="jsonData" className="block text-sm font-medium text-gray-700 mb-2">
              JSON Data
            </label>
            <textarea
              id="jsonData"
              value={jsonData}
              onChange={(e) => {
                setJsonData(e.target.value)
                setValidationResult(null)
              }}
              placeholder="Enter your JSON data here..."
              className="w-full h-64 p-3 border border-gray-300 rounded-md font-mono text-sm"
            />
          </div>

          {/* Validation Result */}
          {validationResult && (
            <div className={`p-3 rounded-md ${
              validationResult.valid 
                ? 'bg-green-50 border border-green-200' 
                : 'bg-red-50 border border-red-200'
            }`}>
              <div className={`text-sm font-medium ${
                validationResult.valid ? 'text-green-800' : 'text-red-800'
              }`}>
                {validationResult.valid ? '✓ Valid JSON' : '✗ Invalid JSON'}
              </div>
              {validationResult.error && (
                <div className="text-sm text-red-600 mt-1">{validationResult.error}</div>
              )}
              {validationResult.message && (
                <div className="text-sm text-green-600 mt-1">{validationResult.message}</div>
              )}
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={validateJson}
              className="px-4 py-2 bg-gray-600 text-white rounded hover:bg-gray-700 text-sm"
            >
              Validate JSON
            </button>
          </div>
        </div>
      </div>

      {/* OpenRouter Configuration Section */}
      <div className="bg-white p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-3">OpenRouter AI Configuration</h3>
        
        <div className="space-y-4">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label htmlFor="openrouterApiKey" className="block text-sm font-medium text-gray-700 mb-1">
                OpenRouter API Key
              </label>
              <input
                type="password"
                id="openrouterApiKey"
                value={openrouterApiKey}
                onChange={(e) => setOpenrouterApiKey(e.target.value)}
                placeholder="Enter your OpenRouter API key"
                className="w-full p-2 border border-gray-300 rounded text-sm"
              />
              <p className="text-xs text-gray-500 mt-1">
                Get your API key from <a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-blue-600 hover:underline">openrouter.ai/keys</a>
              </p>
            </div>
            
            <div>
              <label htmlFor="openrouterModel" className="block text-sm font-medium text-gray-700 mb-1">
                AI Model
              </label>
              <select
                id="openrouterModel"
                value={openrouterModel}
                onChange={(e) => setOpenrouterModel(e.target.value)}
                className="w-full p-2 border border-gray-300 rounded text-sm"
              >
                <option value="anthropic/claude-3.5-sonnet">Claude 3.5 Sonnet</option>
                <option value="anthropic/claude-3-haiku">Claude 3 Haiku</option>
                <option value="openai/gpt-4o">GPT-4o</option>
                <option value="openai/gpt-4o-mini">GPT-4o Mini</option>
                <option value="meta-llama/llama-3.1-8b-instruct">Llama 3.1 8B</option>
                <option value="meta-llama/llama-3.1-70b-instruct">Llama 3.1 70B</option>
              </select>
            </div>
          </div>
          
          <div className="flex items-center gap-4">
            <button
              onClick={saveOpenRouterConfig}
              className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 text-sm"
            >
              Save Configuration
            </button>
            
            {configSaveStatus && (
              <div className={`text-sm ${
                configSaveStatus.success ? 'text-green-600' : 'text-red-600'
              }`}>
                {configSaveStatus.success ? '✓' : '✗'} {configSaveStatus.message}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Ingestion Configuration Section */}
      <div className="bg-white p-4 rounded-lg shadow">
        <h3 className="text-lg font-medium text-gray-900 mb-3">Ingestion Configuration</h3>
        
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <div>
            <label htmlFor="autoExecute" className="flex items-center">
              <input
                type="checkbox"
                id="autoExecute"
                checked={autoExecute}
                onChange={(e) => setAutoExecute(e.target.checked)}
                className="mr-2"
              />
              <span className="text-sm font-medium text-gray-700">Auto Execute Mutations</span>
            </label>
          </div>
          
          <div>
            <label htmlFor="trustDistance" className="block text-sm font-medium text-gray-700 mb-1">
              Trust Distance
            </label>
            <input
              type="number"
              id="trustDistance"
              value={trustDistance}
              onChange={(e) => setTrustDistance(parseInt(e.target.value) || 0)}
              min="0"
              className="w-full p-2 border border-gray-300 rounded text-sm"
            />
          </div>
          
          <div>
            <label htmlFor="pubKey" className="block text-sm font-medium text-gray-700 mb-1">
              Public Key
            </label>
            <input
              type="text"
              id="pubKey"
              value={pubKey}
              onChange={(e) => setPubKey(e.target.value)}
              className="w-full p-2 border border-gray-300 rounded text-sm"
            />
          </div>
        </div>
      </div>

      {/* Action Section */}
      <div className="bg-white p-4 rounded-lg shadow">
        <div className="flex justify-between items-center">
          <div>
            <h3 className="text-lg font-medium text-gray-900">Process Ingestion</h3>
            <p className="text-sm text-gray-600 mt-1">
              AI will analyze your data and automatically create schemas or map to existing ones
            </p>
          </div>
          
          <button
            onClick={processIngestion}
            disabled={isLoading || !jsonData.trim()}
            className={`px-6 py-2 rounded font-medium ${
              isLoading || !jsonData.trim()
                ? 'bg-gray-300 text-gray-500 cursor-not-allowed'
                : 'bg-primary text-white hover:bg-primary-dark'
            }`}
          >
            {isLoading ? 'Processing...' : 'Process Data'}
          </button>
        </div>
      </div>

      {/* Help Section */}
      <div className="bg-blue-50 p-4 rounded-lg border border-blue-200">
        <h4 className="text-sm font-medium text-blue-900 mb-2">How it works:</h4>
        <ol className="text-sm text-blue-800 space-y-1">
          <li>1. Configure your OpenRouter API key and model</li>
          <li>2. Enter your JSON data or use a sample</li>
          <li>3. Configure ingestion settings (optional)</li>
          <li>4. Click "Process Data" to start AI analysis</li>
          <li>5. AI will create schemas and store your data automatically</li>
        </ol>
      </div>
    </div>
  )
}

export default IngestionTab