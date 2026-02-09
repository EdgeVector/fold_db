import { useState, useEffect } from 'react'
import { ingestionClient } from '../../api/clients'

function AiConfigSettings({ configSaveStatus, setConfigSaveStatus, onClose }) {
  const [aiProvider, setAiProvider] = useState('OpenRouter')
  const [openrouterApiKey, setOpenrouterApiKey] = useState('')
  const [openrouterModel, setOpenrouterModel] = useState('anthropic/claude-3.5-sonnet')
  const [openrouterBaseUrl, setOpenrouterBaseUrl] = useState('https://openrouter.ai/api/v1')
  const [ollamaModel, setOllamaModel] = useState('llama3')
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState('http://192.168.1.226:11434')
  const [showAdvanced, setShowAdvanced] = useState(false)

  useEffect(() => {
    loadAiConfig()
  }, [])

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

  return {
    aiProvider,
    saveAiConfig,
    content: (
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
    )
  }
}

export default AiConfigSettings
