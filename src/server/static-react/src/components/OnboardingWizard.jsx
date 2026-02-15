import { useState, useEffect, useCallback } from 'react'
import { ingestionClient, systemClient } from '../api/clients'
import { BROWSER_CONFIG } from '../constants/config'
import SelectField from './form/SelectField'

const TOTAL_STEPS = 4

const OPENROUTER_MODELS = [
  { value: 'google/gemini-2.0-flash-001', label: 'Gemini 2.0 Flash (Recommended)' },
  { value: 'anthropic/claude-sonnet-4', label: 'Claude Sonnet 4' },
  { value: 'openai/gpt-4o-mini', label: 'GPT-4o Mini' },
  { value: 'meta-llama/llama-3.1-8b-instruct', label: 'Llama 3.1 8B' },
]

const OLLAMA_MODELS = [
  { value: 'llama3.1:8b', label: 'Llama 3.1 8B (Recommended)' },
  { value: 'mistral:7b', label: 'Mistral 7B' },
  { value: 'gemma2:9b', label: 'Gemma 2 9B' },
]

function ProgressBar({ currentStep }) {
  return (
    <div className="px-6 pt-5 pb-2">
      <div className="flex gap-1.5">
        {Array.from({ length: TOTAL_STEPS }, (_, i) => (
          <div
            key={i}
            className={`h-1.5 flex-1 rounded-full ${
              i < currentStep ? 'bg-primary' : 'bg-border'
            }`}
          />
        ))}
      </div>
      <p className="text-xs text-tertiary mt-2">Step {currentStep} of {TOTAL_STEPS}</p>
    </div>
  )
}

function WelcomeStep({ onNext }) {
  return (
    <div>
      <h2 className="text-xl font-semibold mb-3">Welcome to FoldDB</h2>
      <p className="text-secondary mb-4">
        FoldDB is a schema-based database with AI-powered data ingestion. Let's get you set up in a few quick steps.
      </p>
      <ul className="space-y-2 text-sm text-secondary mb-6">
        <li className="flex items-start gap-2">
          <span className="text-primary mt-0.5">1.</span>
          <span>Configure your AI provider for ingestion and search</span>
        </li>
        <li className="flex items-start gap-2">
          <span className="text-primary mt-0.5">2.</span>
          <span>Review your storage configuration</span>
        </li>
      </ul>
      <button onClick={onNext} className="btn-primary w-full">
        Get Started
      </button>
    </div>
  )
}

function ConfigureAiStep({ onNext, onSkip, onConfigSaved }) {
  const [provider, setProvider] = useState('OpenRouter')
  const [model, setModel] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [ollamaModel, setOllamaModel] = useState('')
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [saveResult, setSaveResult] = useState(null)
  const [alreadyConfigured, setAlreadyConfigured] = useState(false)

  useEffect(() => {
    let cancelled = false
    ingestionClient.getConfig().then(response => {
      if (cancelled) return
      if (response.success && response.data) {
        const cfg = response.data
        setProvider(cfg.provider || 'OpenRouter')
        if (cfg.openrouter?.model) setModel(cfg.openrouter.model)
        if (cfg.ollama?.model) setOllamaModel(cfg.ollama.model)
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
        model: provider === 'Ollama' ? (ollamaModel || OLLAMA_MODELS[0].value) : '',
        base_url: 'http://localhost:11434',
      },
    }
    try {
      const response = await ingestionClient.saveConfig(config)
      if (response.success) {
        setSaveResult('success')
        onConfigSaved()
        setTimeout(() => onNext(), 1000)
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
    return (
      <div className="flex items-center justify-center py-8">
        <div className="w-5 h-5 border-2 border-border border-t-primary rounded-full animate-spin" />
      </div>
    )
  }

  return (
    <div>
      <h2 className="text-xl font-semibold mb-1">Configure AI Provider</h2>
      <p className="text-secondary text-sm mb-4">
        FoldDB uses AI for data ingestion and search. Choose a provider below.
      </p>

      {alreadyConfigured && (
        <div className="bg-green-50 border border-green-200 px-3 py-2 mb-4 text-green-800 text-sm rounded">
          AI provider is already configured. You can update it or skip this step.
        </div>
      )}

      <SelectField
        name="provider"
        label="Provider"
        value={provider}
        options={[
          { value: 'OpenRouter', label: 'OpenRouter (Cloud)' },
          { value: 'Ollama', label: 'Ollama (Local)' },
        ]}
        onChange={setProvider}
      />

      {provider === 'OpenRouter' && (
        <div className="mt-3 space-y-3">
          <SelectField
            name="model"
            label="Model"
            value={model || OPENROUTER_MODELS[0].value}
            options={OPENROUTER_MODELS}
            onChange={setModel}
          />
          <div>
            <label className="block text-sm font-medium text-primary mb-1">API Key</label>
            <input
              type="password"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
              placeholder={alreadyConfigured ? '***configured***' : 'sk-or-...'}
              className="input w-full"
              data-testid="api-key-input"
            />
            <a
              href="https://openrouter.ai/keys"
              target="_blank"
              rel="noopener noreferrer"
              className="text-xs text-blue-600 hover:underline mt-1 inline-block"
            >
              Get an API key from OpenRouter
            </a>
          </div>
        </div>
      )}

      {provider === 'Ollama' && (
        <div className="mt-3 space-y-3">
          <SelectField
            name="ollama-model"
            label="Model"
            value={ollamaModel || OLLAMA_MODELS[0].value}
            options={OLLAMA_MODELS}
            onChange={setOllamaModel}
          />
          <div className="bg-surface-secondary border border-border px-3 py-2 text-sm rounded">
            <p className="font-medium mb-1">Setup</p>
            <p className="text-secondary">
              Make sure Ollama is running locally. Pull a model with:
            </p>
            <code className="block mt-1 text-xs bg-surface px-2 py-1 border border-border rounded">
              ollama pull {ollamaModel || OLLAMA_MODELS[0].value}
            </code>
          </div>
        </div>
      )}

      {saveResult === 'success' && (
        <div className="mt-3 bg-green-50 border border-green-200 px-3 py-2 text-green-800 text-sm rounded">
          Configuration saved successfully!
        </div>
      )}
      {saveResult === 'error' && (
        <div className="mt-3 bg-red-50 border border-red-200 px-3 py-2 text-red-800 text-sm rounded">
          Failed to save configuration. Please try again.
        </div>
      )}

      <div className="flex gap-3 mt-5">
        <button
          onClick={handleSave}
          disabled={saving || (provider === 'OpenRouter' && !apiKey && !alreadyConfigured)}
          className="btn-primary flex-1"
        >
          {saving ? 'Saving...' : 'Save & Continue'}
        </button>
        <button onClick={onSkip} className="btn-secondary flex-1">
          Skip for Now
        </button>
      </div>
    </div>
  )
}

function StorageInfoStep({ onNext, storageInfo }) {
  const isLocal = !storageInfo || storageInfo.type === 'local'
  const mode = isLocal ? 'Local' : 'Cloud'

  return (
    <div>
      <h2 className="text-xl font-semibold mb-1">Storage Configuration</h2>
      <p className="text-secondary text-sm mb-4">
        Here's how your data is currently being stored.
      </p>

      <div className="bg-surface-secondary border border-border px-4 py-3 rounded mb-4">
        <div className="flex items-center gap-3">
          <span className="text-2xl">{isLocal ? '\uD83D\uDCBB' : '\u2601\uFE0F'}</span>
          <div>
            <p className="font-medium">{mode} Storage</p>
            <p className="text-secondary text-sm">
              {isLocal
                ? 'Data is stored locally using Sled embedded database.'
                : `Data is stored in AWS (DynamoDB + S3) in ${storageInfo.region || 'us-west-2'}.`}
            </p>
          </div>
        </div>
      </div>

      <div className="bg-surface-secondary border border-border px-3 py-2 text-sm rounded">
        <p className="font-medium mb-1">Switching Modes</p>
        <p className="text-secondary">
          Start the server with <code className="bg-surface px-1 border border-border rounded">./run.sh --local</code> for
          local mode or <code className="bg-surface px-1 border border-border rounded">./run.sh</code> for cloud mode.
        </p>
      </div>

      <button onClick={onNext} className="btn-primary w-full mt-5">
        Continue
      </button>
    </div>
  )
}

function DoneStep({ onComplete }) {
  const features = [
    {
      title: 'Smart Folder Sync',
      description: 'Point FoldDB at a folder and let AI categorize and ingest your files automatically.',
    },
    {
      title: 'AI Query',
      description: 'Search your data using natural language queries powered by your configured AI provider.',
    },
    {
      title: 'Advanced Features',
      description: 'Explore schemas, mutations, transforms, and native indexing for full control over your data.',
    },
  ]

  return (
    <div>
      <h2 className="text-xl font-semibold mb-1">You're All Set!</h2>
      <p className="text-secondary text-sm mb-4">
        FoldDB is ready to use. Here are some things you can try:
      </p>

      <div className="space-y-3 mb-6">
        {features.map((f) => (
          <div key={f.title} className="bg-surface-secondary border border-border px-3 py-2 rounded">
            <p className="font-medium text-sm">{f.title}</p>
            <p className="text-secondary text-xs mt-0.5">{f.description}</p>
          </div>
        ))}
      </div>

      <button onClick={onComplete} className="btn-primary w-full">
        Start Using FoldDB
      </button>
    </div>
  )
}

export default function OnboardingWizard({ isOpen, onClose }) {
  const [currentStep, setCurrentStep] = useState(1)
  const [aiWasConfigured, setAiWasConfigured] = useState(false)
  const [storageInfo, setStorageInfo] = useState(null)

  // Fetch storage info on mount
  useEffect(() => {
    if (!isOpen) return
    systemClient.getDatabaseConfig().then(response => {
      if (response.success && response.data) {
        setStorageInfo(response.data)
      }
    }).catch(() => {})
  }, [isOpen])

  const handleComplete = useCallback(() => {
    localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED, '1')
    onClose()
  }, [onClose])

  const handleSkipTutorial = useCallback(() => {
    localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED, '1')
    onClose()
  }, [onClose])

  if (!isOpen) return null

  const goNext = () => setCurrentStep(s => Math.min(s + 1, TOTAL_STEPS))
  const goBack = () => setCurrentStep(s => Math.max(s - 1, 1))

  const renderStep = () => {
    switch (currentStep) {
      case 1:
        return <WelcomeStep onNext={goNext} />
      case 2:
        return (
          <ConfigureAiStep
            onNext={goNext}
            onSkip={goNext}
            onConfigSaved={() => setAiWasConfigured(true)}
          />
        )
      case 3:
        return <StorageInfoStep onNext={goNext} storageInfo={storageInfo} />
      case 4:
        return <DoneStep onComplete={handleComplete} />
      default:
        return null
    }
  }

  return (
    <div className="modal-overlay" onClick={(e) => e.stopPropagation()}>
      <div className="modal" onClick={(e) => e.stopPropagation()} style={{ maxWidth: '480px' }}>
        <ProgressBar currentStep={currentStep} />

        <div className="modal-body px-6 py-4">
          {renderStep()}
        </div>

        <div className="modal-footer px-6 py-3 flex items-center justify-between border-t border-border">
          <div>
            {currentStep > 1 && currentStep < 4 && (
              <button onClick={goBack} className="btn-secondary btn-sm">
                Back
              </button>
            )}
          </div>
          <div>
            {currentStep < 4 && (
              <button
                onClick={handleSkipTutorial}
                className="text-sm text-tertiary hover:text-secondary bg-transparent border-none cursor-pointer"
              >
                Skip Tutorial
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
