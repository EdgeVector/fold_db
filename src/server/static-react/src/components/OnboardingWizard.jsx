import { useState, useEffect, useCallback } from 'react'
import { ingestionClient, systemClient } from '../api/clients'
import { BROWSER_CONFIG } from '../constants/config'

const TOTAL_STEPS = 4

const OPENROUTER_MODELS = [
  { value: 'google/gemini-2.0-flash-001', label: 'Gemini 2.0 Flash' },
  { value: 'anthropic/claude-sonnet-4', label: 'Claude Sonnet 4' },
  { value: 'openai/gpt-4o-mini', label: 'GPT-4o Mini' },
  { value: 'meta-llama/llama-3.1-8b-instruct', label: 'Llama 3.1 8B' },
]

const OLLAMA_MODELS = [
  { value: 'llama3.1:8b', label: 'Llama 3.1 8B' },
  { value: 'mistral:7b', label: 'Mistral 7B' },
  { value: 'gemma2:9b', label: 'Gemma 2 9B' },
]

// Gruvbox-warm palette matching fold_db_website
const colors = {
  bg: '#282828',
  bgElevated: '#3c3836',
  border: '#504945',
  text: '#ebdbb2',
  textBright: '#fbf1c7',
  dim: '#928374',
  orange: '#fe8019',
  yellow: '#fabd2f',
  green: '#b8bb26',
  blue: '#83a598',
  purple: '#d3869b',
  red: '#fb4934',
  link: '#8ec07c',
}

const styles = {
  overlay: {
    position: 'fixed', top: 0, left: 0, width: '100%', height: '100%',
    background: 'rgba(0,0,0,0.7)', zIndex: 1000,
    display: 'flex', alignItems: 'center', justifyContent: 'center',
  },
  modal: {
    background: colors.bgElevated, border: `1px solid ${colors.border}`,
    padding: 0, maxWidth: '520px', width: '90%', maxHeight: '85vh',
    overflowY: 'auto', color: colors.text,
    fontFamily: "'IBM Plex Mono', monospace", fontSize: '14px', lineHeight: '1.5',
  },
  header: {
    padding: '24px 24px 0',
  },
  body: {
    padding: '0 24px 24px',
  },
  footer: {
    padding: '12px 24px', borderTop: `1px solid ${colors.border}`,
    display: 'flex', alignItems: 'center', justifyContent: 'space-between',
  },
  card: {
    border: `1px solid ${colors.border}`, padding: '12px', marginBottom: '8px',
  },
  label: {
    padding: '1px 6px', fontWeight: 700, color: colors.bg, display: 'inline-block',
    fontSize: '12px', marginBottom: '4px',
  },
  btnPrimary: {
    background: 'none', border: `1px solid ${colors.orange}`, color: colors.orange,
    padding: '6px 16px', cursor: 'pointer', fontFamily: 'inherit', fontSize: 'inherit',
    width: '100%', textAlign: 'center',
  },
  btnSecondary: {
    background: 'none', border: `1px solid ${colors.border}`, color: colors.dim,
    padding: '6px 16px', cursor: 'pointer', fontFamily: 'inherit', fontSize: 'inherit',
  },
  btnLink: {
    background: 'none', border: 'none', color: colors.dim, cursor: 'pointer',
    fontFamily: 'inherit', fontSize: '12px', padding: 0,
  },
  input: {
    background: colors.bg, border: `1px solid ${colors.border}`, color: colors.text,
    padding: '6px 8px', width: '100%', fontFamily: 'inherit', fontSize: 'inherit',
    outline: 'none',
  },
  select: {
    background: colors.bg, border: `1px solid ${colors.border}`, color: colors.text,
    padding: '6px 8px', width: '100%', fontFamily: 'inherit', fontSize: 'inherit',
    outline: 'none', appearance: 'auto',
  },
  pre: {
    background: colors.bg, border: `1px solid ${colors.border}`,
    padding: '8px', margin: '8px 0', fontFamily: 'inherit', fontSize: '12px',
  },
}

function ProgressBar({ currentStep }) {
  const segments = Array.from({ length: TOTAL_STEPS }, (_, i) => (
    <div
      key={i}
      style={{
        flex: 1, height: '3px',
        background: i < currentStep ? colors.yellow : colors.border,
      }}
    />
  ))
  return (
    <div style={styles.header}>
      <div style={{ display: 'flex', gap: '4px' }}>{segments}</div>
      <p style={{ fontSize: '12px', color: colors.dim, marginTop: '8px' }}>
        Step {currentStep} of {TOTAL_STEPS}
      </p>
    </div>
  )
}

function WelcomeStep({ onNext }) {
  return (
    <div>
      <p style={{ fontSize: '1.4em', fontWeight: 700, color: colors.orange, margin: '0.3em 0' }}>
        Welcome to Fold DB
      </p>
      <p>
        Your personal data node with AI-powered ingestion. Let&apos;s get you set up.
      </p>

      <div style={{ margin: '16px 0' }}>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.green }}>01 AI PROVIDER</span></p>
          <p style={{ margin: '4px 0' }}>Configure OpenRouter or local Ollama for ingestion and search</p>
        </div>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.blue }}>02 STORAGE</span></p>
          <p style={{ margin: '4px 0' }}>Choose local Sled or Exemem cloud storage</p>
        </div>
      </div>

      <button
        onClick={onNext}
        style={styles.btnPrimary}
        onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
        onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
      >
        [Get Started]
      </button>
    </div>
  )
}

function ConfigureAiStep({ onNext, onSkip, onConfigSaved }) {
  const [provider, setProvider] = useState('OpenRouter')
  const [model, setModel] = useState('')
  const [apiKey, setApiKey] = useState('')
  const [ollamaModel, setOllamaModel] = useState('')
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434')
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
        model: provider === 'Ollama' ? (ollamaModel || OLLAMA_MODELS[0].value) : '',
        base_url: ollamaUrl,
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
    return <p style={{ color: colors.dim, textAlign: 'center', padding: '24px 0' }}>Loading configuration...</p>
  }

  const models = provider === 'OpenRouter' ? OPENROUTER_MODELS : OLLAMA_MODELS
  const currentModel = provider === 'OpenRouter' ? (model || OPENROUTER_MODELS[0].value) : (ollamaModel || OLLAMA_MODELS[0].value)

  return (
    <div>
      <h2 style={{ fontSize: 'inherit', fontWeight: 700, margin: '0 0 4px' }}>
        <span style={{ color: colors.green }}>CONFIGURE AI</span>{' '}
        <span style={{ color: colors.dim }}>Provider setup</span>
      </h2>
      <p>FoldDB uses AI for data ingestion and search.</p>

      {alreadyConfigured && (
        <div style={{ ...styles.card, borderColor: colors.green, marginTop: '12px' }}>
          <p><span style={{ ...styles.label, background: colors.green }}>CONFIGURED</span></p>
          <p style={{ margin: '4px 0' }}>AI provider is already set up. Update below or skip.</p>
        </div>
      )}

      <div style={{ marginTop: '16px' }}>
        <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Provider</p>
        <select
          value={provider}
          onChange={e => setProvider(e.target.value)}
          style={styles.select}
          data-testid="provider-select"
        >
          <option value="OpenRouter">OpenRouter (Cloud)</option>
          <option value="Ollama">Ollama (Local)</option>
        </select>
      </div>

      <div style={{ marginTop: '12px' }}>
        <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Model</p>
        <select
          value={currentModel}
          onChange={e => provider === 'OpenRouter' ? setModel(e.target.value) : setOllamaModel(e.target.value)}
          style={styles.select}
          data-testid="model-select"
        >
          {models.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
        </select>
      </div>

      {provider === 'OpenRouter' && (
        <div style={{ marginTop: '12px' }}>
          <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>API Key</p>
          <input
            type="password"
            value={apiKey}
            onChange={e => setApiKey(e.target.value)}
            placeholder={alreadyConfigured ? '***configured***' : 'sk-or-...'}
            style={styles.input}
            data-testid="api-key-input"
          />
          <p style={{ marginTop: '4px' }}>
            <a
              href="https://openrouter.ai/keys"
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: colors.link, fontSize: '12px', textDecoration: 'none' }}
            >
              [Get API key from OpenRouter]
            </a>
          </p>
        </div>
      )}

      {provider === 'Ollama' && (
        <>
          <div style={{ marginTop: '12px' }}>
            <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Ollama URL</p>
            <input
              type="text"
              value={ollamaUrl}
              onChange={e => setOllamaUrl(e.target.value)}
              placeholder="http://localhost:11434"
              style={styles.input}
            />
            <p style={{ color: colors.dim, fontSize: '12px', marginTop: '4px' }}>
              Use a LAN address (e.g. http://192.168.1.100:11434) for a remote instance
            </p>
          </div>
          <div style={{ ...styles.pre, marginTop: '12px' }}>
            <p style={{ color: colors.textBright, fontWeight: 700 }}>Setup</p>
            <p style={{ color: colors.dim }}>Make sure Ollama is running:</p>
            <p style={{ color: colors.yellow, marginTop: '4px' }}>$ ollama pull {currentModel}</p>
          </div>
        </>
      )}

      {saveResult === 'success' && (
        <p style={{ color: colors.green, marginTop: '8px' }}>Configuration saved successfully!</p>
      )}
      {saveResult === 'error' && (
        <p style={{ color: colors.red, marginTop: '8px' }}>Failed to save. Please try again.</p>
      )}

      <div style={{ display: 'flex', gap: '8px', marginTop: '16px' }}>
        <button
          onClick={handleSave}
          disabled={saving || (provider === 'OpenRouter' && !apiKey && !alreadyConfigured)}
          style={{
            ...styles.btnPrimary, flex: 1,
            opacity: (saving || (provider === 'OpenRouter' && !apiKey && !alreadyConfigured)) ? 0.4 : 1,
          }}
          onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
          onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
        >
          {saving ? 'Saving...' : '[Save & Continue]'}
        </button>
        <button onClick={onSkip} style={{ ...styles.btnSecondary, flex: 1 }}>
          [Skip]
        </button>
      </div>
    </div>
  )
}

function StorageConfigStep({ onNext, onSkip, storageInfo }) {
  const currentIsLocal = !storageInfo || storageInfo.type === 'local'
  const [storageType, setStorageType] = useState(currentIsLocal ? 'local' : 'exemem')
  const [localPath, setLocalPath] = useState(storageInfo?.path || 'data')
  const [exememUrl, setExememUrl] = useState(storageInfo?.api_url || '')
  const [exememKey, setExememKey] = useState('')
  const [saving, setSaving] = useState(false)
  const [saveResult, setSaveResult] = useState(null)

  const handleSave = async () => {
    setSaving(true)
    setSaveResult(null)
    const setup = {
      storage: storageType === 'local'
        ? { type: 'local', path: localPath }
        : { type: 'exemem', api_url: exememUrl, api_key: exememKey },
    }
    try {
      const response = await systemClient.applySetup(setup)
      if (response.success) {
        setSaveResult('success')
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

  const canSave = storageType === 'local'
    ? localPath.trim() !== ''
    : exememUrl.trim() !== '' && exememKey.trim() !== ''

  return (
    <div>
      <h2 style={{ fontSize: 'inherit', fontWeight: 700, margin: '0 0 4px' }}>
        <span style={{ color: colors.blue }}>STORAGE</span>{' '}
        <span style={{ color: colors.dim }}>Backend configuration</span>
      </h2>
      <p>Choose where FoldDB stores your data.</p>

      <div style={{ marginTop: '16px' }}>
        <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Storage Backend</p>
        <select
          value={storageType}
          onChange={e => setStorageType(e.target.value)}
          style={styles.select}
          data-testid="storage-type-select"
        >
          <option value="local">Local (Sled)</option>
          <option value="exemem">Exemem Cloud</option>
        </select>
      </div>

      {storageType === 'local' && (
        <div style={{ marginTop: '12px' }}>
          <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Data Directory</p>
          <input
            type="text"
            value={localPath}
            onChange={e => setLocalPath(e.target.value)}
            placeholder="data"
            style={styles.input}
            data-testid="storage-path-input"
          />
          <p style={{ color: colors.dim, fontSize: '12px', marginTop: '4px' }}>
            Relative to the server working directory
          </p>
        </div>
      )}

      {storageType === 'exemem' && (
        <>
          <div style={{ marginTop: '12px' }}>
            <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Exemem API URL</p>
            <input
              type="text"
              value={exememUrl}
              onChange={e => setExememUrl(e.target.value)}
              placeholder="https://api.exemem.com"
              style={styles.input}
              data-testid="exemem-url-input"
            />
          </div>
          <div style={{ marginTop: '12px' }}>
            <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>API Key</p>
            <input
              type="password"
              value={exememKey}
              onChange={e => setExememKey(e.target.value)}
              placeholder="Enter your API key"
              style={styles.input}
              data-testid="exemem-key-input"
            />
          </div>
        </>
      )}

      {saveResult === 'success' && (
        <p style={{ color: colors.green, marginTop: '8px' }}>Storage configuration saved!</p>
      )}
      {saveResult === 'error' && (
        <p style={{ color: colors.red, marginTop: '8px' }}>Failed to save. Please try again.</p>
      )}

      <div style={{ display: 'flex', gap: '8px', marginTop: '16px' }}>
        <button
          onClick={handleSave}
          disabled={saving || !canSave}
          style={{
            ...styles.btnPrimary, flex: 1,
            opacity: (saving || !canSave) ? 0.4 : 1,
          }}
          onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
          onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
        >
          {saving ? 'Saving...' : '[Save & Continue]'}
        </button>
        <button onClick={onSkip} style={{ ...styles.btnSecondary, flex: 1 }}>
          [Skip]
        </button>
      </div>
    </div>
  )
}


function DoneStep({ onComplete }) {
  return (
    <div>
      <p style={{ fontSize: '1.4em', fontWeight: 700, color: colors.orange, margin: '0.3em 0' }}>
        You&apos;re all set.
      </p>
      <p>Your Personal Data Node is ready. Here&apos;s what you can do:</p>

      <div style={{ margin: '16px 0' }}>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.green }}>SMART FOLDER SYNC</span></p>
          <p style={{ margin: '4px 0' }}>Point FoldDB at a folder and let AI categorize and ingest your files.</p>
        </div>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.blue }}>AI QUERY</span></p>
          <p style={{ margin: '4px 0' }}>Search your data using natural language queries.</p>
        </div>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.purple }}>SCHEMAS & MUTATIONS</span></p>
          <p style={{ margin: '4px 0' }}>Explore schemas, transforms, and native indexing for full control.</p>
        </div>
      </div>

      <button
        onClick={onComplete}
        style={styles.btnPrimary}
        onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
        onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
      >
        [Start Using FoldDB]
      </button>
    </div>
  )
}

export default function OnboardingWizard({ isOpen, onClose, userHash }) {
  const [currentStep, setCurrentStep] = useState(1)
  const [aiWasConfigured, setAiWasConfigured] = useState(false)
  const [storageInfo, setStorageInfo] = useState(null)

  useEffect(() => {
    if (!isOpen) return
    systemClient.getDatabaseConfig().then(response => {
      if (response.success && response.data) {
        setStorageInfo(response.data)
      }
    }).catch(() => {})
  }, [isOpen])

  const handleComplete = useCallback(() => {
    if (userHash) {
      localStorage.setItem(`${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${userHash}`, '1')
    }
    onClose()
  }, [onClose, userHash])

  const handleSkipTutorial = useCallback(() => {
    if (userHash) {
      localStorage.setItem(`${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${userHash}`, '1')
    }
    onClose()
  }, [onClose, userHash])

  if (!isOpen) return null

  const goNext = () => setCurrentStep(s => Math.min(s + 1, TOTAL_STEPS))
  const goBack = () => setCurrentStep(s => Math.max(s - 1, 1))

  const renderStep = () => {
    switch (currentStep) {
      case 1: return <WelcomeStep onNext={goNext} />
      case 2: return <ConfigureAiStep onNext={goNext} onSkip={goNext} onConfigSaved={() => setAiWasConfigured(true)} />
      case 3: return <StorageConfigStep onNext={goNext} onSkip={goNext} storageInfo={storageInfo} />
      case 4: return <DoneStep onComplete={handleComplete} />
      default: return null
    }
  }

  return (
    <div style={styles.overlay} onClick={e => e.stopPropagation()}>
      <div style={styles.modal} onClick={e => e.stopPropagation()}>
        <ProgressBar currentStep={currentStep} />
        <div style={styles.body}>{renderStep()}</div>
        <div style={styles.footer}>
          <div>
            {currentStep > 1 && currentStep < TOTAL_STEPS && (
              <button onClick={goBack} style={styles.btnSecondary}>
                [Back]
              </button>
            )}
          </div>
          <div>
            {currentStep < TOTAL_STEPS && (
              <button onClick={handleSkipTutorial} style={styles.btnLink}>
                Skip Tutorial
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
