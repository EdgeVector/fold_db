import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'

function AiConfigSettings({ configSaveStatus, setConfigSaveStatus, onClose, onConfigSaved }) {
  const [aiProvider, setAiProvider] = useState('OpenRouter')
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [openrouterModel, setOpenrouterModel] = useState('anthropic/claude-3.5-sonnet')
  const [openrouterBaseUrl, setOpenrouterBaseUrl] = useState('https://openrouter.ai/api/v1')
  const [ollamaModel, setOllamaModel] = useState('llama3')
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState('http://localhost:11434')
  const [showAdvanced, setShowAdvanced] = useState(false)

  useEffect(() => { loadAiConfig() }, [])

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
    } catch (error) { console.error('Failed to load AI config:', error) }
  }

  const saveAiConfig = async () => {
    try {
      const config = {
        provider: aiProvider,
        openrouter: { api_key: openrouterApiKey, model: openrouterModel, base_url: openrouterBaseUrl },
        ollama: { model: ollamaModel, base_url: ollamaBaseUrl },
      }
      const response = await ingestionClient.saveConfig(config)
      if (response.success) {
        setConfigSaveStatus({ success: true, message: 'Configuration saved successfully' })
        if (onConfigSaved) onConfigSaved()
        setTimeout(() => { setConfigSaveStatus(null); onClose() }, 1500)
      } else {
        setConfigSaveStatus({ success: false, message: 'Failed to save configuration' })
      }
    } catch (error) {
      setConfigSaveStatus({ success: false, message: error.message || 'Failed to save configuration' })
    }
    setTimeout(() => setConfigSaveStatus(null), 3000)
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
                <option value="anthropic/claude-3.5-sonnet">Claude 3.5 Sonnet</option>
                <option value="anthropic/claude-3.5-haiku">Claude 3.5 Haiku</option>
                <option value="openai/gpt-4o">GPT-4o</option>
                <option value="openai/gpt-4o-mini">GPT-4o Mini</option>
                <option value="google/gemini-2.0-flash-exp">Gemini 2.0 Flash</option>
                <option value="deepseek/deepseek-chat">DeepSeek Chat</option>
              </select>
            ) : (
              <>
                <select value={ollamaModel} onChange={(e) => setOllamaModel(e.target.value)} className="select">
                  <option value="llama3.3">Llama 3.3 (70B)</option>
                  <option value="llama3.2">Llama 3.2 (3B)</option>
                  <option value="llama3">Llama 3 (8B)</option>
                  <option value="mistral">Mistral (7B)</option>
                  <option value="deepseek-coder-v2">DeepSeek Coder V2</option>
                </select>
                <p className="text-xs text-secondary mt-1">Pull model: <code className="text-gruvbox-blue">ollama pull {ollamaModel}</code></p>
              </>
            )}
          </div>
        </div>

        {aiProvider === 'OpenRouter' && (
          <div>
            <label className="label">API Key <span className="text-xs text-secondary">(<a href="https://openrouter.ai/keys" target="_blank" rel="noopener noreferrer" className="text-gruvbox-blue hover:underline">get key</a>)</span></label>
            <input type="password" value={openrouterApiKey} onChange={(e) => setOpenrouterApiKey(e.target.value)} placeholder="sk-or-..." className="input" />
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

export default AiConfigSettings
