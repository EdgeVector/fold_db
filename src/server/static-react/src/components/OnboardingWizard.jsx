import { useState, useEffect, useCallback, useRef } from 'react'
import { BROWSER_CONFIG } from '../constants/config'
import ConfigureAiStep from './onboarding/ConfigureAiStep'

const TOTAL_STEPS = 3

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
