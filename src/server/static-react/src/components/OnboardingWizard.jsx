import { useState, useEffect, useCallback, useRef } from 'react'
import { ingestionClient, llmQueryClient } from '../api/clients'
import { BROWSER_CONFIG } from '../constants/config'

const TOTAL_STEPS = 6

const OPENROUTER_MODELS = [
  { value: 'google/gemini-2.5-flash', label: 'Gemini 2.5 Flash' },
  { value: 'anthropic/claude-sonnet-4.6', label: 'Claude Sonnet 4.6' },
  { value: 'google/gemini-3.1-pro', label: 'Gemini 3.1 Pro' },
  { value: 'openai/gpt-4.1-mini', label: 'GPT-4.1 Mini' },
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

function PrimaryButton({ onClick, disabled, children }) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      style={{ ...styles.btnPrimary, opacity: disabled ? 0.4 : 1 }}
      onMouseEnter={e => { if (!disabled) { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow } }}
      onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
    >
      {children}
    </button>
  )
}

// Step 1: Welcome
function WelcomeStep({ onNext }) {
  return (
    <div>
      <p style={{ fontSize: '1.4em', fontWeight: 700, color: colors.orange, margin: '0.3em 0' }}>
        Welcome to FoldDB
      </p>
      <p>
        Your personal AI database. Drop in any file, AI organizes it, search everything in plain English.
      </p>
      <p style={{ color: colors.dim, fontSize: '12px', marginTop: '4px' }}>
        Takes ~2 minutes to set up.
      </p>

      <div style={{ margin: '16px 0' }}>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.green }}>01 AI SETUP</span></p>
          <p style={{ margin: '4px 0' }}>Configure your AI provider for ingestion and search</p>
        </div>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.blue }}>02 TRY IT</span></p>
          <p style={{ margin: '4px 0' }}>Drop a file and ask it a question &mdash; see the magic</p>
        </div>
        <div style={styles.card}>
          <p><span style={{ ...styles.label, background: colors.purple }}>03 GO</span></p>
          <p style={{ margin: '4px 0' }}>Point FoldDB at a folder and let it work</p>
        </div>
      </div>

      <PrimaryButton onClick={onNext}>[Get Started]</PrimaryButton>
    </div>
  )
}

// Step 2: Configure AI
function ConfigureAiStep({ onNext, onSkip, onConfigSaved }) {
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
  const advanceTimeoutRef = useRef(null)
  const ollamaFetchTimeoutRef = useRef(null)

  useEffect(() => {
    return () => {
      if (advanceTimeoutRef.current) clearTimeout(advanceTimeoutRef.current)
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
        onConfigSaved()
        advanceTimeoutRef.current = setTimeout(() => onNext(), 1000)
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

  const currentModel = provider === 'OpenRouter' ? (model || OPENROUTER_MODELS[0].value) : ollamaModel
  const canSave = saving || (provider === 'OpenRouter' && !apiKey && !alreadyConfigured)

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
        {provider === 'OpenRouter' ? (
          <select
            value={currentModel}
            onChange={e => setModel(e.target.value)}
            style={styles.select}
            data-testid="model-select"
          >
            {OPENROUTER_MODELS.map(m => <option key={m.value} value={m.value}>{m.label}</option>)}
          </select>
        ) : ollamaModelsLoading ? (
          <div style={{ ...styles.input, display: 'flex', alignItems: 'center', color: colors.dim }}>Loading models...</div>
        ) : ollamaModels.length > 0 ? (
          <select
            value={ollamaModel}
            onChange={e => setOllamaModel(e.target.value)}
            style={styles.select}
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
            style={styles.input}
            data-testid="model-select"
          />
        )}
        {provider === 'Ollama' && ollamaModelsError && (
          <p style={{ color: colors.red, fontSize: '12px', marginTop: '4px' }}>{ollamaModelsError}</p>
        )}
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
          disabled={canSave}
          style={{
            ...styles.btnPrimary, flex: 1,
            opacity: canSave ? 0.4 : 1,
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

// Step 3: First File (NEW - "Hello World" moment)
function FirstFileStep({ onNext, onSkip, onFileIngested }) {
  const [isDragging, setIsDragging] = useState(false)
  const [selectedFile, setSelectedFile] = useState(null)
  const [isUploading, setIsUploading] = useState(false)
  const [uploadResult, setUploadResult] = useState(null)
  const fileInputRef = useRef(null)

  const handleDragEnter = (e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(true)
  }

  const handleDragLeave = (e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
  }

  const handleDrop = (e) => {
    e.preventDefault()
    e.stopPropagation()
    setIsDragging(false)
    const file = e.dataTransfer.files[0]
    if (file) {
      setSelectedFile(file)
      handleUpload(file)
    }
  }

  const handleFileSelect = (e) => {
    const file = e.target.files[0]
    if (file) {
      setSelectedFile(file)
      handleUpload(file)
    }
  }

  const handleUpload = async (file) => {
    setIsUploading(true)
    setUploadResult(null)
    try {
      const response = await ingestionClient.uploadFile(file, {
        autoExecute: true,
        trustDistance: 0,
        pubKey: 'default',
      })
      if (response.success && response.data) {
        const result = {
          schemaName: response.data.schema_name,
          newSchema: response.data.new_schema_created,
          mutationsExecuted: response.data.mutations_executed,
        }
        setUploadResult({ success: true, ...result })
        onFileIngested(result)
      } else {
        setUploadResult({ success: false, error: response.error || 'Upload failed' })
      }
    } catch (err) {
      setUploadResult({ success: false, error: err.message || 'Upload failed' })
    } finally {
      setIsUploading(false)
    }
  }

  const dropZoneStyle = {
    border: `2px dashed ${isDragging ? colors.yellow : colors.border}`,
    padding: '32px 16px',
    textAlign: 'center',
    cursor: 'pointer',
    marginTop: '16px',
    transition: 'border-color 0.2s',
    background: isDragging ? 'rgba(250,189,47,0.05)' : 'transparent',
  }

  return (
    <div>
      <h2 style={{ fontSize: 'inherit', fontWeight: 700, margin: '0 0 4px' }}>
        <span style={{ color: colors.blue }}>FIRST FILE</span>{' '}
        <span style={{ color: colors.dim }}>See AI in action</span>
      </h2>
      <p>Drop a file and watch FoldDB&apos;s AI process it into structured data.</p>
      <p style={{ color: colors.dim, fontSize: '12px' }}>
        Try a PDF, text file, JSON, CSV &mdash; anything with data you care about.
      </p>

      {!uploadResult && !isUploading && (
        <div
          style={dropZoneStyle}
          onDragEnter={handleDragEnter}
          onDragOver={handleDragEnter}
          onDragLeave={handleDragLeave}
          onDrop={handleDrop}
          onClick={() => fileInputRef.current?.click()}
        >
          <input
            ref={fileInputRef}
            type="file"
            onChange={handleFileSelect}
            style={{ display: 'none' }}
          />
          <p style={{ color: isDragging ? colors.yellow : colors.textBright, fontSize: '1.1em', margin: '0 0 8px' }}>
            {isDragging ? 'Drop it here' : 'Drag & drop a file here'}
          </p>
          <p style={{ color: colors.dim, fontSize: '12px', margin: 0 }}>
            or click to browse
          </p>
        </div>
      )}

      {isUploading && (
        <div style={{ ...styles.card, borderColor: colors.yellow, marginTop: '16px', textAlign: 'center' }}>
          <p style={{ color: colors.yellow, margin: '8px 0' }}>Processing {selectedFile?.name}...</p>
          <div style={{ display: 'flex', gap: '4px', justifyContent: 'center', margin: '12px 0' }}>
            {[0, 1, 2].map(i => (
              <div key={i} style={{
                width: '8px', height: '8px', borderRadius: '50%', background: colors.yellow,
                animation: `pulse 1.2s ease-in-out ${i * 0.2}s infinite`,
              }} />
            ))}
          </div>
          <style>{`@keyframes pulse { 0%, 100% { opacity: 0.3; } 50% { opacity: 1; } }`}</style>
          <p style={{ color: colors.dim, fontSize: '12px', margin: '4px 0' }}>
            AI is reading your file, detecting the schema, and structuring the data...
          </p>
        </div>
      )}

      {uploadResult?.success && (
        <div style={{ ...styles.card, borderColor: colors.green, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.green }}>INGESTED</span></p>
          <p style={{ margin: '8px 0', color: colors.textBright }}>{selectedFile?.name}</p>
          <div style={styles.pre}>
            <p style={{ margin: '2px 0' }}>
              <span style={{ color: colors.dim }}>Schema:</span>{' '}
              <span style={{ color: colors.yellow }}>{uploadResult.schemaName}</span>
              {uploadResult.newSchema && <span style={{ color: colors.green, marginLeft: '8px', fontSize: '12px' }}>(new)</span>}
            </p>
            <p style={{ margin: '2px 0' }}>
              <span style={{ color: colors.dim }}>Records:</span>{' '}
              <span style={{ color: colors.textBright }}>{uploadResult.mutationsExecuted || 1}</span>
            </p>
          </div>
          <p style={{ color: colors.dim, fontSize: '12px', margin: '8px 0 0' }}>
            Your data is now structured and searchable.
          </p>
        </div>
      )}

      {uploadResult && !uploadResult.success && (
        <div style={{ ...styles.card, borderColor: colors.red, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.red }}>ERROR</span></p>
          <p style={{ margin: '4px 0', color: colors.red, fontSize: '12px' }}>{uploadResult.error}</p>
          <button
            onClick={() => { setUploadResult(null); setSelectedFile(null) }}
            style={{ ...styles.btnSecondary, marginTop: '8px', width: '100%', textAlign: 'center' }}
          >
            [Try Another File]
          </button>
        </div>
      )}

      <div style={{ display: 'flex', gap: '8px', marginTop: '16px' }}>
        {uploadResult?.success && (
          <PrimaryButton onClick={onNext}>[Continue]</PrimaryButton>
        )}
        {!uploadResult?.success && !isUploading && (
          <button onClick={onSkip} style={{ ...styles.btnSecondary, width: '100%', textAlign: 'center' }}>
            [Skip for now]
          </button>
        )}
      </div>
    </div>
  )
}

// Step 4: AI Query Demo (NEW)
function AiQueryDemoStep({ onNext, onSkip, ingestedFile }) {
  const [query, setQuery] = useState('')
  const [isQuerying, setIsQuerying] = useState(false)
  const [answer, setAnswer] = useState(null)
  const [queryError, setQueryError] = useState(null)

  useEffect(() => {
    if (ingestedFile?.schemaName) {
      setQuery(`What information is in my ${ingestedFile.schemaName.replace(/_/g, ' ')} data?`)
    }
  }, [ingestedFile])

  const handleQuery = async () => {
    if (!query.trim()) return
    setIsQuerying(true)
    setAnswer(null)
    setQueryError(null)
    try {
      const response = await llmQueryClient.agentQuery({
        query: query.trim(),
        max_iterations: 10,
      })
      if (response.data?.answer) {
        setAnswer(response.data.answer)
      } else {
        setQueryError('No answer returned. Make sure AI is configured.')
      }
    } catch (err) {
      setQueryError(err.message || 'Query failed')
    } finally {
      setIsQuerying(false)
    }
  }

  const hasFile = !!ingestedFile

  return (
    <div>
      <h2 style={{ fontSize: 'inherit', fontWeight: 700, margin: '0 0 4px' }}>
        <span style={{ color: colors.purple }}>AI QUERY</span>{' '}
        <span style={{ color: colors.dim }}>Ask your data anything</span>
      </h2>
      {hasFile ? (
        <p>Your file is ingested. Try asking it a question in plain English.</p>
      ) : (
        <p>Search your data using natural language. Try it out below.</p>
      )}

      <div style={{ marginTop: '16px' }}>
        <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Your question</p>
        <input
          type="text"
          value={query}
          onChange={e => setQuery(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && !isQuerying && handleQuery()}
          placeholder={hasFile ? `Ask about your ${ingestedFile.schemaName}...` : 'Ask anything about your data...'}
          style={styles.input}
          disabled={isQuerying}
        />
      </div>

      <div style={{ marginTop: '12px' }}>
        <button
          onClick={handleQuery}
          disabled={isQuerying || !query.trim()}
          style={{
            ...styles.btnPrimary,
            opacity: (isQuerying || !query.trim()) ? 0.4 : 1,
          }}
          onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
          onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
        >
          {isQuerying ? 'Thinking...' : '[Ask]'}
        </button>
      </div>

      {isQuerying && (
        <div style={{ textAlign: 'center', margin: '16px 0' }}>
          <div style={{ display: 'flex', gap: '4px', justifyContent: 'center' }}>
            {[0, 1, 2].map(i => (
              <div key={i} style={{
                width: '8px', height: '8px', borderRadius: '50%', background: colors.purple,
                animation: `pulse 1.2s ease-in-out ${i * 0.2}s infinite`,
              }} />
            ))}
          </div>
          <style>{`@keyframes pulse { 0%, 100% { opacity: 0.3; } 50% { opacity: 1; } }`}</style>
          <p style={{ color: colors.dim, fontSize: '12px', marginTop: '8px' }}>
            AI is searching and analyzing your data...
          </p>
        </div>
      )}

      {answer && (
        <div style={{ ...styles.card, borderColor: colors.green, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.green }}>ANSWER</span></p>
          <p style={{ margin: '8px 0', whiteSpace: 'pre-wrap', fontSize: '13px' }}>{answer}</p>
        </div>
      )}

      {queryError && (
        <div style={{ ...styles.card, borderColor: colors.red, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.red }}>ERROR</span></p>
          <p style={{ margin: '4px 0', color: colors.red, fontSize: '12px' }}>{queryError}</p>
        </div>
      )}

      <div style={{ marginTop: '16px' }}>
        {answer ? (
          <PrimaryButton onClick={onNext}>[Continue]</PrimaryButton>
        ) : (
          <button onClick={onSkip} style={{ ...styles.btnSecondary, width: '100%', textAlign: 'center' }}>
            [Skip]
          </button>
        )}
      </div>
    </div>
  )
}

// Step 5: Smart Folder (optional)
function SmartFolderStep({ onNext, onSkip }) {
  const [folderPath, setFolderPath] = useState('')
  const [isScanning, setIsScanning] = useState(false)
  const [scanResult, setScanResult] = useState(null)

  const handleScan = async () => {
    if (!folderPath.trim()) return
    setIsScanning(true)
    setScanResult(null)
    try {
      const response = await ingestionClient.smartFolderScan(folderPath.trim())
      if (response.success && response.data) {
        setScanResult({
          success: true,
          totalFiles: response.data.total_files,
          personalFiles: response.data.recommendations?.filter(r => r.should_process)?.length || 0,
        })
      } else {
        setScanResult({ success: false, error: response.error || 'Scan failed' })
      }
    } catch (err) {
      setScanResult({ success: false, error: err.message || 'Scan failed' })
    } finally {
      setIsScanning(false)
    }
  }

  return (
    <div>
      <h2 style={{ fontSize: 'inherit', fontWeight: 700, margin: '0 0 4px' }}>
        <span style={{ color: colors.green }}>SMART FOLDER</span>{' '}
        <span style={{ color: colors.dim }}>Automatic sync (optional)</span>
      </h2>
      <p>
        Point FoldDB at a folder and it will automatically find and ingest your personal data files.
      </p>

      <div style={{ marginTop: '16px' }}>
        <p style={{ color: colors.textBright, fontWeight: 700, marginBottom: '4px' }}>Folder path</p>
        <input
          type="text"
          value={folderPath}
          onChange={e => setFolderPath(e.target.value)}
          placeholder="/Users/you/Documents"
          style={styles.input}
          disabled={isScanning}
        />
        <p style={{ color: colors.dim, fontSize: '12px', marginTop: '4px' }}>
          AI will scan for personal data files (photos, documents, notes, etc.)
        </p>
      </div>

      {!scanResult && (
        <div style={{ display: 'flex', gap: '8px', marginTop: '16px' }}>
          <button
            onClick={handleScan}
            disabled={isScanning || !folderPath.trim()}
            style={{
              ...styles.btnPrimary, flex: 1,
              opacity: (isScanning || !folderPath.trim()) ? 0.4 : 1,
            }}
            onMouseEnter={e => { e.target.style.color = colors.yellow; e.target.style.borderColor = colors.yellow }}
            onMouseLeave={e => { e.target.style.color = colors.orange; e.target.style.borderColor = colors.orange }}
          >
            {isScanning ? 'Scanning...' : '[Scan Folder]'}
          </button>
          <button onClick={onSkip} style={{ ...styles.btnSecondary, flex: 1 }}>
            [Skip]
          </button>
        </div>
      )}

      {scanResult?.success && (
        <div style={{ ...styles.card, borderColor: colors.green, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.green }}>SCANNED</span></p>
          <div style={styles.pre}>
            <p style={{ margin: '2px 0' }}>
              <span style={{ color: colors.dim }}>Total files:</span>{' '}
              <span style={{ color: colors.textBright }}>{scanResult.totalFiles}</span>
            </p>
            <p style={{ margin: '2px 0' }}>
              <span style={{ color: colors.dim }}>Personal data:</span>{' '}
              <span style={{ color: colors.yellow }}>{scanResult.personalFiles} files</span>
            </p>
          </div>
          <p style={{ color: colors.dim, fontSize: '12px', margin: '8px 0 0' }}>
            You can start the full ingestion from the Smart Folder tab after setup.
          </p>
          <div style={{ marginTop: '12px' }}>
            <PrimaryButton onClick={onNext}>[Continue]</PrimaryButton>
          </div>
        </div>
      )}

      {scanResult && !scanResult.success && (
        <div style={{ ...styles.card, borderColor: colors.red, marginTop: '16px' }}>
          <p><span style={{ ...styles.label, background: colors.red }}>ERROR</span></p>
          <p style={{ margin: '4px 0', color: colors.red, fontSize: '12px' }}>{scanResult.error}</p>
          <div style={{ display: 'flex', gap: '8px', marginTop: '12px' }}>
            <button onClick={() => setScanResult(null)} style={{ ...styles.btnSecondary, flex: 1, textAlign: 'center' }}>
              [Try Again]
            </button>
            <button onClick={onSkip} style={{ ...styles.btnSecondary, flex: 1, textAlign: 'center' }}>
              [Skip]
            </button>
          </div>
        </div>
      )}
    </div>
  )
}

// Step 6: Done
function DoneStep({ onComplete }) {
  return (
    <div>
      <p style={{ fontSize: '1.4em', fontWeight: 700, color: colors.orange, margin: '0.3em 0' }}>
        You&apos;re all set.
      </p>
      <p>Your personal AI database is ready. Here&apos;s what you can do:</p>

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
          <p><span style={{ ...styles.label, background: colors.purple }}>FILE UPLOAD</span></p>
          <p style={{ margin: '4px 0' }}>Drop in individual files for instant AI-powered ingestion.</p>
        </div>
      </div>

      <div style={{ ...styles.card, borderColor: colors.blue, marginTop: '8px' }}>
        <p style={{ color: colors.blue, fontWeight: 700, margin: '0 0 4px', fontSize: '12px' }}>
          WANT MORE?
        </p>
        <p style={{ margin: '4px 0', fontSize: '13px' }}>
          Upgrade to <span style={{ color: colors.textBright }}>Exemem Cloud</span> for sync, backup, API access, and app development.
        </p>
        <p style={{ margin: '4px 0' }}>
          <a
            href="https://exemem.com"
            target="_blank"
            rel="noopener noreferrer"
            style={{ color: colors.link, fontSize: '12px', textDecoration: 'none' }}
          >
            [Learn more about Exemem Cloud]
          </a>
        </p>
      </div>

      <div style={{ marginTop: '16px' }}>
        <PrimaryButton onClick={onComplete}>[Start Using FoldDB]</PrimaryButton>
      </div>
    </div>
  )
}

export default function OnboardingWizard({ isOpen, onClose, userHash }) {
  const [currentStep, setCurrentStep] = useState(1)
  const [, setAiWasConfigured] = useState(false)
  const [ingestedFile, setIngestedFile] = useState(null)

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
      case 3: return <FirstFileStep onNext={goNext} onSkip={goNext} onFileIngested={setIngestedFile} />
      case 4: return <AiQueryDemoStep onNext={goNext} onSkip={goNext} ingestedFile={ingestedFile} />
      case 5: return <SmartFolderStep onNext={goNext} onSkip={goNext} />
      case 6: return <DoneStep onComplete={handleComplete} />
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
