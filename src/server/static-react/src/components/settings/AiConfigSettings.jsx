import { useState, useEffect, useRef, useCallback } from 'react'
import { ingestionClient } from '../../api/clients'
import { useAppSelector, useAppDispatch } from '../../store/hooks'
import { selectIngestionConfig, saveIngestionConfig } from '../../store/ingestionSlice'

function useAiConfig({ configSaveStatus, setConfigSaveStatus, onClose }) {
  const dispatch = useAppDispatch()
  const savedConfig = useAppSelector(selectIngestionConfig)
  const [aiProvider, setAiProvider] = useState('OpenRouter')
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [hasEnvKey, setHasEnvKey] = useState(false)
  const [openrouterModel, setOpenrouterModel] = useState('google/gemini-2.5-flash')
  const [openrouterBaseUrl, setOpenrouterBaseUrl] = useState('https://openrouter.ai/api/v1')
  const [ollamaModel, setOllamaModel] = useState('')
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState('http://localhost:11434')
  const [ollamaModels, setOllamaModels] = useState([])
  const [ollamaModelsLoading, setOllamaModelsLoading] = useState(false)
  const [ollamaModelsError, setOllamaModelsError] = useState(null)
  const [showAdvanced, setShowAdvanced] = useState(false)
  const statusTimeoutRef = useRef(null)
  const ollamaFetchTimeoutRef = useRef(null)

  useEffect(() => {
    return () => {
      if (statusTimeoutRef.current) clearTimeout(statusTimeoutRef.current)
      if (ollamaFetchTimeoutRef.current) clearTimeout(ollamaFetchTimeoutRef.current)
    }
  }, [])

  const fetchOllamaModels = useCallback(async (url) => {
    if (!url) return
    setOllamaModelsLoading(true)
    setOllamaModelsError(null)
    setOllamaModels([])
    try {
      const response = await ingestionClient.listOllamaModels(url)
      const data = response?.data ?? response
      const models = data?.models ?? []
      const error = data?.error
      setOllamaModels(models)
      if (error) {
        setOllamaModelsError(error)
      } else if (models.length === 0) {
        setOllamaModelsError('No models found. Run: ollama pull <model>')
      } else {
        setOllamaModelsError(null)
        // Auto-select first model if none currently selected
        setOllamaModel(prev => {
          if (!prev || !models.some(m => m.name === prev)) return models[0].name
          return prev
        })
      }
    } catch (err) {
      setOllamaModels([])
      setOllamaModelsError(`Could not connect to Ollama: ${err?.message || err}`)
    } finally {
      setOllamaModelsLoading(false)
    }
  }, [])

  // Fetch Ollama models when provider is Ollama and URL changes (debounced)
  useEffect(() => {
    if (aiProvider !== 'Ollama') return
    if (ollamaFetchTimeoutRef.current) clearTimeout(ollamaFetchTimeoutRef.current)
    ollamaFetchTimeoutRef.current = setTimeout(() => fetchOllamaModels(ollamaBaseUrl), 500)
    return () => { if (ollamaFetchTimeoutRef.current) clearTimeout(ollamaFetchTimeoutRef.current) }
  }, [aiProvider, ollamaBaseUrl, fetchOllamaModels])

  // Initialize form state from Redux store
  useEffect(() => {
    if (savedConfig) {
      const apiKey = savedConfig.openrouter?.api_key || ''
      if (apiKey === '***configured***') {
        setHasEnvKey(true)
        setOpenrouterApiKey('')
      } else {
        setHasEnvKey(false)
        setOpenrouterApiKey(apiKey)
      }
      setOpenrouterModel(savedConfig.openrouter?.model || 'google/gemini-2.5-flash')
      setOpenrouterBaseUrl(savedConfig.openrouter?.base_url || 'https://openrouter.ai/api/v1')
      setOllamaModel(savedConfig.ollama?.model || 'llama3.3')
      setOllamaBaseUrl(savedConfig.ollama?.base_url || 'http://localhost:11434')
      setAiProvider(savedConfig.provider || 'OpenRouter')
    }
  }, [savedConfig])

  const saveAiConfig = async () => {
    try {
      const config = {
        provider: aiProvider,
        openrouter: { api_key: openrouterApiKey, model: openrouterModel, base_url: openrouterBaseUrl },
        ollama: { model: ollamaModel, base_url: ollamaBaseUrl },
      }
      await dispatch(saveIngestionConfig(config)).unwrap()
      setConfigSaveStatus({ success: true, message: 'Configuration saved successfully' })
      if (statusTimeoutRef.current) clearTimeout(statusTimeoutRef.current)
      statusTimeoutRef.current = setTimeout(() => { setConfigSaveStatus(null); onClose() }, 1500)
    } catch (error) {
      setConfigSaveStatus({ success: false, message: (error instanceof Error ? error.message : String(error)) || 'Failed to save configuration' })
      if (statusTimeoutRef.current) clearTimeout(statusTimeoutRef.current)
      statusTimeoutRef.current = setTimeout(() => setConfigSaveStatus(null), 3000)
    }
  }

  return {
    aiProvider,
    saveAiConfig,
    content: (
      <div className="space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div>
            <label className="label">Provider</label>
            <select value={aiProvider} onChange={(e) => setAiProvider(e.target.value)} className="select">
              <option value="OpenRouter">OpenRouter</option>
              <option value="Ollama">Ollama</option>
            </select>
          </div>
          <div>
            <label className="label">Model</label>
            {aiProvider === 'OpenRouter' ? (
              <select value={openrouterModel} onChange={(e) => setOpenrouterModel(e.target.value)} className="select">
                <option value="google/gemini-2.5-flash">Gemini 2.5 Flash</option>
                <option value="anthropic/claude-sonnet-4.6">Claude Sonnet 4.6</option>
                <option value="google/gemini-3.1-pro">Gemini 3.1 Pro</option>
                <option value="openai/gpt-4.1-mini">GPT-4.1 Mini</option>
                <option value="openai/gpt-4.1">GPT-4.1</option>
                <option value="deepseek/deepseek-chat-v3-0324">DeepSeek V3</option>
              </select>
            ) : (
              <>
                {ollamaModelsLoading ? (
                  <div className="input flex items-center text-sm text-secondary">Loading models...</div>
                ) : ollamaModels.length > 0 ? (
                  <select value={ollamaModel} onChange={(e) => setOllamaModel(e.target.value)} className="select">
                    {ollamaModels.map(m => (
                      <option key={m.name} value={m.name}>{m.name} ({(m.size / 1e9).toFixed(1)} GB)</option>
                    ))}
                  </select>
                ) : (
                  <input
                    type="text"
                    value={ollamaModel}
                    onChange={(e) => setOllamaModel(e.target.value)}
                    placeholder="e.g. llama3.3"
                    className="input"
                  />
                )}
                {ollamaModelsError && (
                  <p className="text-xs text-gruvbox-red mt-1">{ollamaModelsError}</p>
                )}
                {ollamaModel && !ollamaModelsError && (
                  <p className="text-xs text-secondary mt-1">Pull model: <code className="text-gruvbox-blue">ollama pull {ollamaModel}</code></p>
                )}
              </>
            )}
          </div>
        </div>

        {aiProvider === 'OpenRouter' && (
          <div>
            <label className="label">API Key <span className="text-xs text-secondary">(<a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-gruvbox-blue hover:underline">get key</a>)</span></label>
            {hasEnvKey && !openrouterApiKey ? (
              <>
                <div className="input flex items-center text-sm text-gruvbox-green">Configured via environment variable</div>
                <p className="text-xs text-secondary mt-1">Enter a new key below to override, or leave as-is to use the environment variable.</p>
              </>
            ) : (
              <input type="password" value={openrouterApiKey} onChange={(e) => setOpenrouterApiKey(e.target.value)} placeholder="sk-or-..." className="input" />
            )}
          </div>
        )}

        {aiProvider === 'Ollama' && (
          <div>
            <label className="label">Ollama URL</label>
            <input
              type="text"
              value={ollamaBaseUrl}
              onChange={(e) => setOllamaBaseUrl(e.target.value)}
              placeholder="http://localhost:11434"
              className="input"
            />
            <p className="text-xs text-secondary mt-1">Use a LAN address for a remote instance (e.g. http://192.168.1.100:11434)</p>
          </div>
        )}

        <div>
          <button onClick={() => setShowAdvanced(!showAdvanced)} className="text-sm text-secondary hover:text-primary flex items-center gap-1">
            <span>{showAdvanced ? '▼' : '▶'}</span> Advanced
          </button>
          {showAdvanced && (
            <div className="mt-3 space-y-3 pl-4 border-l-2 border-border">
              {aiProvider === 'OpenRouter' && (
                <div>
                  <label className="label">Base URL</label>
                  <input
                    type="text"
                    value={openrouterBaseUrl}
                    onChange={(e) => setOpenrouterBaseUrl(e.target.value)}
                    className="input"
                  />
                </div>
              )}
            </div>
          )}
        </div>

        {configSaveStatus && (
          <div className={`p-3 card ${configSaveStatus.success ? 'card-success text-gruvbox-green' : 'card-error text-gruvbox-red'}`}>
            <span className="text-sm font-medium">{configSaveStatus.success ? '✓' : '✗'} {configSaveStatus.message}</span>
          </div>
        )}
      </div>
    )
  }
}

export default useAiConfig
