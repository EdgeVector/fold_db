import { useState, useEffect, useCallback, useRef } from 'react'
import { ingestionClient } from '../api/clients'
import { BROWSER_CONFIG } from '../constants/config'
import { useAppDispatch } from '../store/hooks'
import { fetchIngestionConfig } from '../store/ingestionSlice'

const TOTAL_STEPS = 3

const OPENROUTER_MODELS = [
  { value: 'google/gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
  { value: 'anthropic/claude-sonnet-4.6', label: 'Claude Sonnet 4.6' },
  { value: 'google/gemini-3.1-pro', label: 'Gemini 3.1 Pro' },
  { value: 'openai/gpt-4.1-mini', label: 'GPT-4.1 Mini' },
  { value: 'openai/gpt-4.1', label: 'GPT-4.1' },
  { value: 'deepseek/deepseek-chat-v3-0324', label: 'DeepSeek V3' },
]

function ProgressBar({ currentStep }) {
  const segments = Array.from({ length: TOTAL_STEPS }, (_, i) => (
    <div
      key={i}
      className={i < currentStep ? 'h-[3px] flex-1 bg-gruvbox-yellow' : 'h-[3px] flex-1 bg-border'}
    />
  ))
  return (
    <div className="px-6 pt-6">
      <div className="flex gap-1">{segments}</div>
      <p className="text-xs text-secondary mt-2">
        Step {currentStep} of {TOTAL_STEPS}
      </p>
    </div>
  )
}

// Step 1: Welcome
function WelcomeStep({ onNext }) {
  return (
    <div>
      <p className="text-xl font-bold text-gruvbox-orange mt-1 mb-1">
        Welcome to FoldDB
      </p>
      <p className="text-primary">
        Your personal AI database. Drop in any file, AI organizes it, search everything in plain English.
      </p>
      <p className="text-secondary text-xs mt-1">
        One quick setup step and you&apos;re ready to go.
      </p>

      <div className="mt-4 space-y-2">
        <div className="card p-3">
          <p><span className="badge badge-success">AI SETUP</span></p>
          <p className="text-primary mt-1">Configure your AI provider for ingestion and search</p>
        </div>
        <div className="card p-3">
          <p><span className="badge badge-info">FILE INGESTION</span></p>
          <p className="text-primary mt-1">Drop files in and AI structures your data automatically</p>
        </div>
        <div className="card p-3">
          <p><span className="badge badge-warning">AI SEARCH</span></p>
          <p className="text-primary mt-1">Ask questions about your data in plain English</p>
        </div>
      </div>

      <button className="btn-primary w-full text-center mt-4" onClick={onNext}>
        Get Started
      </button>
    </div>
  )
}

// Step 2: Configure AI
function ConfigureAiStep({ onNext, onSkip }) {
  const dispatch = useAppDispatch()
  const [provider, setProvider] = useState('OpenRouter')
  const [model, setModel] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [ollamaModel, setOllamaModel] = useState('')
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434')
  const [ollamaModels, setOllamaModels] = useState([])
  const [ollamaModelsLoading, setOllamaModelsLoading] = useState(false)
  const [ollamaModelsError, setOllamaModelsError] = useState(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [saveResult, setSaveResult] = useState(null)
  const [alreadyConfigured, setAlreadyConfigured] = useState(false)
  const ollamaFetchTimeoutRef = useRef(null)

  useEffect(() => {
    return () => {
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

  useEffect(() => {
    if (provider !== 'Ollama') return
    if (ollamaFetchTimeoutRef.current) clearTimeout(ollamaFetchTimeoutRef.current)
    ollamaFetchTimeoutRef.current = setTimeout(() => fetchOllamaModels(ollamaUrl), 500)
    return () => { if (ollamaFetchTimeoutRef.current) clearTimeout(ollamaFetchTimeoutRef.current) }
  }, [provider, ollamaUrl, fetchOllamaModels])

  useEffect(() => {
    let cancelled = false
    ingestionClient.getConfig().then(response => {
      if (cancelled) return
      if (response.success && response.data) {
        const cfg = response.data
        setProvider(cfg.provider || 'OpenRouter')
        if (cfg.openrouter?.model) setModel(cfg.openrouter.model)
        if (cfg.ollama?.model) setOllamaModel(cfg.ollama.model)
        if (cfg.ollama?.base_url) setOllamaUrl(cfg.ollama.base_url)
        if (cfg.openrouter?.api_key && cfg.openrouter.api_key.includes('configured')) {
          setAlreadyConfigured(true)
        }
      }
      setLoading(false)
    }).catch(() => {
      if (!cancelled) setLoading(false)
    })
    return () => { cancelled = true }
  }, [])

  const handleSave = async () => {
    setSaving(true)
    setSaveResult(null)
    const config = {
      provider,
      openrouter: {
        api_key: provider === 'OpenRouter' ? apiKey : '',
        model: provider === 'OpenRouter' ? (model || OPENROUTER_MODELS[0].value) : '',
        base_url: 'https://openrouter.ai/api/v1',
      },
      ollama: {
        model: provider === 'Ollama' ? (ollamaModel || (ollamaModels[0]?.name ?? '')) : '',
        base_url: ollamaUrl,
      },
    }
    try {
      const response = await ingestionClient.saveConfig(config)
      if (response.success) {
        setSaveResult('success')
        dispatch(fetchIngestionConfig())
      } else {
        setSaveResult('error')
      }
    } catch {
      setSaveResult('error')
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return <p className="text-secondary text-center py-6">Loading configuration...</p>
  }

  const currentModel = provider === 'OpenRouter' ? (model || OPENROUTER_MODELS[0].value) : ollamaModel
  const canSave = saving || (provider === 'OpenRouter' && !apiKey && !alreadyConfigured)

  return (
    <div>
      <h2 className="text-sm font-bold mb-1">
        <span className="text-gruvbox-green">AI SETUP</span>{' '}
        <span className="text-secondary">Provider configuration</span>
      </h2>
      <p className="text-primary">FoldDB uses AI for data ingestion and search.</p>

      {alreadyConfigured && (
        <div className="card-success p-3 mt-3">
          <p><span className="badge badge-success">CONFIGURED</span></p>
          <p className="text-primary mt-1">AI provider is already set up. Update below or skip.</p>
        </div>
      )}

      <div className="mt-4">
        <p className="label">Provider</p>
        <select
          value={provider}
          onChange={e => setProvider(e.target.value)}
          className="select"
          data-testid="provider-select"
        >
          <option value="OpenRouter">OpenRouter (Cloud)</option>
          <option value="Ollama">Ollama (Local)</option>
        </select>
      </div>

      <div className="mt-3">
        <p className="label">Model</p>
        {provider === 'OpenRouter' ? (
          <select
            value={currentModel}
            onChange={e => setModel(e.target.value)}
            className="select"
            data-testid="model-select"
          >
            {OPENROUTER_MODELS.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
          </select>
        ) : ollamaModelsLoading ? (
          <div className="input flex items-center text-secondary">Loading models...</div>
        ) : ollamaModels.length > 0 ? (
          <select
            value={ollamaModel}
            onChange={e => setOllamaModel(e.target.value)}
            className="select"
            data-testid="model-select"
          >
            {ollamaModels.map(m => (
              <option key={m.name} value={m.name}>{m.name} ({(m.size / 1e9).toFixed(1)} GB)</option>
            ))}
          </select>
        ) : (
          <input
            type="text"
            value={ollamaModel}
            onChange={e => setOllamaModel(e.target.value)}
            placeholder="e.g. llama3"
            className="input"
            data-testid="model-select"
          />
        )}
        {provider === 'Ollama' && ollamaModelsError && (
          <p className="text-gruvbox-red text-xs mt-1">{ollamaModelsError}</p>
        )}
      </div>

      {provider === 'OpenRouter' && (
        <div className="mt-3">
          <p className="label">API Key</p>
          <input
            type="password"
            value={apiKey}
            onChange={e => setApiKey(e.target.value)}
            placeholder={alreadyConfigured ? '***configured***' : 'sk-or-...'}
            className="input"
            data-testid="api-key-input"
          />
          <p className="mt-1">
            <a
              href="https://openrouter.ai/keys"
              target="_blank"
              rel="noopener noreferrer"
              className="text-gruvbox-link text-xs hover:underline"
            >
              Get API key from OpenRouter
            </a>
          </p>
        </div>
      )}

      {provider === 'Ollama' && (
        <>
          <div className="mt-3">
            <p className="label">Ollama URL</p>
            <input
              type="text"
              value={ollamaUrl}
              onChange={e => setOllamaUrl(e.target.value)}
              placeholder="http://localhost:11434"
              className="input"
            />
            <p className="text-secondary text-xs mt-1">
              Use a LAN address (e.g. http://192.168.1.100:11434) for a remote instance
            </p>
          </div>
          <div className="card p-3 mt-3">
            <p className="font-bold text-primary">Setup</p>
            <p className="text-secondary">Make sure Ollama is running:</p>
            <p className="text-gruvbox-yellow mt-1">$ ollama pull {currentModel}</p>
          </div>
        </>
      )}

      {saveResult === 'success' && (
        <p className="text-gruvbox-green mt-2">Configuration saved successfully!</p>
      )}
      {saveResult === 'error' && (
        <p className="text-gruvbox-red mt-2">Failed to save. Please try again.</p>
      )}

      <div className="flex gap-2 mt-4">
        {saveResult === 'success' ? (
          <button className="btn-primary flex-1 text-center" onClick={onNext}>
            Continue
          </button>
        ) : (
          <>
            <button
              onClick={handleSave}
              disabled={canSave}
              className="btn-primary flex-1 text-center"
            >
              {saving ? 'Saving...' : 'Save & Continue'}
            </button>
            <button onClick={onSkip} className="btn-secondary flex-1 text-center">
              Skip
            </button>
          </>
        )}
      </div>
    </div>
  )
}

// Step 3: Done
function DoneStep({ onComplete }) {
  return (
    <div>
      <p className="text-xl font-bold text-gruvbox-orange mt-1 mb-1">
        You&apos;re all set.
      </p>
      <p className="text-primary">Your personal AI database is ready. Here are some things to try:</p>

      <div className="mt-4 space-y-2">
        <div className="card p-3">
          <p><span className="badge badge-success">FILE UPLOAD</span></p>
          <p className="text-primary mt-1">Drop in a PDF, text file, CSV, or JSON to see AI-powered ingestion in action.</p>
        </div>
        <div className="card p-3">
          <p><span className="badge badge-info">AI SEARCH</span></p>
          <p className="text-primary mt-1">Use the AI Query tab to search your data in plain English.</p>
        </div>
        <div className="card p-3">
          <p><span className="badge badge-warning">SMART FOLDERS</span></p>
          <p className="text-primary mt-1">Point FoldDB at a folder and let it automatically find and ingest your files.</p>
        </div>
      </div>

      <div className="card-info p-3 mt-3">
        <p className="text-gruvbox-blue font-bold text-xs mb-1">
          WANT MORE?
        </p>
        <p className="text-primary text-sm">
          Upgrade to <span className="text-gruvbox-bright">Exemem Cloud</span> for sync, backup, API access, and app development.
        </p>
        <p className="mt-1">
          <a
            href="https://exemem.com"
            target="_blank"
            rel="noopener noreferrer"
            className="text-gruvbox-link text-xs hover:underline"
          >
            Learn more about Exemem Cloud
          </a>
        </p>
      </div>

      <button className="btn-primary w-full text-center mt-4" onClick={onComplete}>
        Start Using FoldDB
      </button>
    </div>
  )
}

export default function OnboardingWizard({ isOpen, onClose, userHash }) {
  const [currentStep, setCurrentStep] = useState(1)
  const modalRef = useRef(null)
  const previousFocusRef = useRef(null)

  const handleDismiss = useCallback(() => {
    if (userHash) {
      localStorage.setItem(`${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${userHash}`, '1')
    }
    onClose()
  }, [onClose, userHash])

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Escape') { handleDismiss(); return }
    if (e.key !== 'Tab') return
    const modal = modalRef.current
    if (!modal) return
    const focusable = modal.querySelectorAll('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')
    if (focusable.length === 0) return
    const first = focusable[0]
    const last = focusable[focusable.length - 1]
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault(); last.focus()
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault(); first.focus()
    }
  }, [handleDismiss])

  useEffect(() => {
    if (!isOpen) return
    previousFocusRef.current = document.activeElement
    const modal = modalRef.current
    if (modal) {
      const firstFocusable = modal.querySelector('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])')
      if (firstFocusable) firstFocusable.focus()
    }
    return () => {
      if (previousFocusRef.current && typeof previousFocusRef.current.focus === 'function') {
        previousFocusRef.current.focus()
      }
    }
  }, [isOpen])

  if (!isOpen) return null

  const goNext = () => setCurrentStep(s => Math.min(s + 1, TOTAL_STEPS))
  const goBack = () => setCurrentStep(s => Math.max(s - 1, 1))

  const renderStep = () => {
    switch (currentStep) {
      case 1: return <WelcomeStep onNext={goNext} />
      case 2: return <ConfigureAiStep onNext={goNext} onSkip={goNext} />
      case 3: return <DoneStep onComplete={handleDismiss} />
      default: return null
    }
  }

  return (
    <div className="modal-overlay" onKeyDown={handleKeyDown}>
      <div
        className="modal max-w-lg"
        ref={modalRef}
        role="dialog"
        aria-modal="true"
        aria-label="Onboarding wizard"
        onClick={e => e.stopPropagation()}
      >
        <ProgressBar currentStep={currentStep} />
        <div className="modal-body">{renderStep()}</div>
        <div className="modal-footer justify-between">
          <div>
            {currentStep > 1 && currentStep < TOTAL_STEPS && (
              <button onClick={goBack} className="btn-secondary">
                Back
              </button>
            )}
          </div>
          <div>
            {currentStep < TOTAL_STEPS && (
              <button onClick={handleDismiss} className="text-secondary text-xs cursor-pointer hover:text-primary bg-transparent border-none">
                Skip Tutorial
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
