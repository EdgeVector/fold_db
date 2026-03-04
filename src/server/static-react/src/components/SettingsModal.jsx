import { useState, useEffect, useRef, useCallback } from 'react'
import KeyManagementTab from './tabs/KeyManagementTab'
import useAiConfig from './settings/AiConfigSettings'
import SchemaServiceSettings from './settings/SchemaServiceSettings'
import useDatabaseConfig from './settings/DatabaseSettings'
import CloudMigrationSettings from './tabs/CloudMigrationSettings'

const NOOP = () => {}

function SettingsModal({ isOpen, onClose, initialTab }) {
  const [activeTab, setActiveTab] = useState(initialTab || 'ai')
  const [configSaveStatus, setConfigSaveStatus] = useState(null)

  const modalRef = useRef(null)
  const previousFocusRef = useRef(null)

  useEffect(() => {
    if (isOpen && initialTab) setActiveTab(initialTab)
  }, [isOpen, initialTab])

  // Focus trap and restore
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

  const handleKeyDown = useCallback((e) => {
    if (e.key === 'Escape') { onClose(); return }
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
  }, [onClose])

  // These custom hooks use React hooks internally and MUST be
  // called unconditionally before the early return below.
  const aiConfig = useAiConfig({ configSaveStatus, setConfigSaveStatus, onClose })
  const dbConfig = useDatabaseConfig({ configSaveStatus, setConfigSaveStatus, onClose })

  if (!isOpen) return null

  const tabs = [
    { id: 'ai', label: 'AI Configuration' },
    { id: 'keys', label: 'Key Management' },
    { id: 'schema-service', label: 'Schema Service' },
    { id: 'database', label: 'Database' },
    { id: 'upgrade-cloud', label: 'Cloud DB' },
  ]

  const handleSave = () => {
    if (activeTab === 'ai') aiConfig.saveAiConfig()
    else if (activeTab === 'database') dbConfig.saveDatabaseConfig()
  }

  return (
    <div className="modal-overlay" onClick={onClose} onKeyDown={handleKeyDown}>
      <div className="modal" ref={modalRef} role="dialog" aria-modal="true" aria-label="Settings" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3 className="text-lg font-medium">Settings</h3>
          <button onClick={onClose} className="btn-secondary btn-sm p-1">
            <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
            </svg>
          </button>
        </div>

        <div className="flex border-b border-border px-6">
          {tabs.map(t => (
            <button
              key={t.id}
              onClick={() => setActiveTab(t.id)}
              className={`tab ${activeTab === t.id ? 'tab-active' : ''}`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="modal-body">
          {activeTab === 'ai' && aiConfig.content}
          {activeTab === 'keys' && <KeyManagementTab onResult={NOOP} />}
          {activeTab === 'schema-service' && <SchemaServiceSettings />}
          {activeTab === 'database' && dbConfig.content}
          {activeTab === 'upgrade-cloud' && <CloudMigrationSettings onClose={onClose} />}
        </div>

        <div className="modal-footer">
          {activeTab === 'ai' || activeTab === 'database' ? (
            <>
              <button onClick={onClose} className="btn-secondary">Cancel</button>
              <button onClick={handleSave} className="btn-primary">
                {activeTab === 'database' ? 'Save and Restart DB' : 'Save Configuration'}
              </button>
            </>
          ) : (
            <button onClick={onClose} className="btn-secondary">Close</button>
          )}
        </div>
      </div>
    </div>
  )
}

export default SettingsModal
