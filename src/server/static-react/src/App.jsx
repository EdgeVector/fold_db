import { useState, useEffect } from 'react'
import { FoldDbProvider } from './components/FoldDbProvider'
import Header from './components/Header'
import Footer from './components/Footer'
import ResultsSection from './components/ResultsSection'
import TabNavigation from './components/TabNavigation'
import SchemaTab from './components/tabs/SchemaTab'
import QueryTab from './components/tabs/QueryTab'
import LlmQueryTab from './components/tabs/LlmQueryTab'
import MutationTab from './components/tabs/MutationTab'
import IngestionTab from './components/tabs/IngestionTab'
import FileUploadTab from './components/tabs/FileUploadTab'
import NativeIndexTab from './components/tabs/NativeIndexTab'
import SmartFolderTab from './components/tabs/SmartFolderTab'
import DataBrowserTab from './components/tabs/DataBrowserTab'
import SettingsModal from './components/SettingsModal'
import OnboardingWizard from './components/OnboardingWizard'
import LogSidebar from './components/LogSidebar'
import { useApprovedSchemas } from './hooks/useApprovedSchemas.js'
import { useIngestionStatus } from './hooks/useIngestionStatus'
import { useAppSelector, useAppDispatch } from './store/hooks'
import { initializeSystemKey, fetchNodePrivateKey, restoreSession } from './store/authSlice'
import LoginPage from './components/LoginPage'
import { DEFAULT_TAB } from './constants'
import { BROWSER_CONFIG } from './constants/config'

// Single lookup for URL hash → tab ID (prevents duplication)
const HASH_TO_TAB = {
  schemas: 'schemas', schema: 'schemas',
  query: 'query', mutation: 'mutation',
  ingestion: 'ingestion', 'file-upload': 'file-upload',
  'native-index': 'native-index',
  'llm-query': 'llm-query', 'ai-query': 'llm-query',
  'smart-folder': 'smart-folder',
  'data-browser': 'data-browser',
}

function resolveTabFromHash() {
  if (typeof window !== 'undefined' && window.location.hash) {
    return HASH_TO_TAB[window.location.hash.slice(1)] || null
  }
  return null
}

export function AppContent() {
  const [activeTab, setActiveTab] = useState(() => resolveTabFromHash() || DEFAULT_TAB)
  const [isSettingsOpen, setIsSettingsOpen] = useState(false)
  const [settingsInitialTab, setSettingsInitialTab] = useState(null)
  const [results, setResults] = useState(null)
  const [setupDismissed, setSetupDismissed] = useState(
    () => localStorage.getItem('folddb_setup_dismissed') === '1'
  )
  const [isOnboardingOpen, setIsOnboardingOpen] = useState(false)
  const userHash = useAppSelector(state => state.auth.user?.hash)
  const [onboardingCompleted, setOnboardingCompleted] = useState(false)

  // Sync activeTab with URL hash changes
  useEffect(() => {
    const handleHashChange = () => {
      const tab = resolveTabFromHash()
      if (tab && tab !== activeTab) {
        setActiveTab(tab)
      }
    }

    window.addEventListener('hashchange', handleHashChange)
    handleHashChange()
    return () => window.removeEventListener('hashchange', handleHashChange)
  }, [activeTab])

  // Redux state and dispatch
  const dispatch = useAppDispatch()
  const { isAuthenticated, isLoading: isAuthLoading } = useAppSelector(state => state.auth)

  // Restore session on mount FIRST - this must run before other effects
  // If no saved credentials, auto-authenticate using the node's public key
  useEffect(() => {
    const userId = localStorage.getItem(BROWSER_CONFIG.STORAGE_KEYS.USER_ID)
    const userHash = localStorage.getItem(BROWSER_CONFIG.STORAGE_KEYS.USER_HASH)
    if (userId && userHash) {
      dispatch(restoreSession({ id: userId, hash: userHash }))
    } else {
      // Auto-authenticate: fetch default identity from backend
      fetch('/api/system/auto-identity')
        .then(res => res.json())
        .then(data => {
          if (data.user_id && data.user_hash) {
            localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.USER_ID, data.user_id)
            localStorage.setItem(BROWSER_CONFIG.STORAGE_KEYS.USER_HASH, data.user_hash)
            dispatch(restoreSession({ id: data.user_id, hash: data.user_hash }))
          }
        })
        .catch(err => console.error('Auto-identity failed:', err))
    }
  }, [dispatch])

  // Initialize system key ONLY after authenticated
  useEffect(() => {
    if (isAuthenticated) {
      dispatch(initializeSystemKey())
    }
  }, [dispatch, isAuthenticated])

  // Fetch node private key ONLY after authenticated
  useEffect(() => {
    if (isAuthenticated) {
      dispatch(fetchNodePrivateKey())
    }
  }, [dispatch, isAuthenticated])

  // Check per-user onboarding status when user hash becomes available
  useEffect(() => {
    if (userHash) {
      const key = `${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${userHash}`
      setOnboardingCompleted(localStorage.getItem(key) === '1')
    }
  }, [userHash])

  // Show onboarding wizard for first-time users
  useEffect(() => {
    if (isAuthenticated && userHash && !onboardingCompleted) {
      const timer = setTimeout(() => setIsOnboardingOpen(true), 500)
      return () => clearTimeout(timer)
    }
  }, [isAuthenticated, userHash, onboardingCompleted])

  const handleOnboardingClose = () => {
    setIsOnboardingOpen(false)
    setOnboardingCompleted(true)
    if (userHash) {
      localStorage.setItem(`${BROWSER_CONFIG.STORAGE_KEYS.ONBOARDING_COMPLETED}_${userHash}`, '1')
    }
    refetchIngestionStatus()
  }

  // Only fetch schemas when authenticated
  const {
    error: schemasError,
    refetch: refetchSchemas
  } = useApprovedSchemas({ enabled: isAuthenticated })

  // Check AI configuration status for setup banner
  const { ingestionStatus, refetchIngestionStatus } = useIngestionStatus()
  const aiConfigured = ingestionStatus?.enabled && ingestionStatus?.configured
  const showSetupBanner = isAuthenticated && ingestionStatus && !aiConfigured && !setupDismissed && !isOnboardingOpen && onboardingCompleted

  const handleTabChange = (tab) => {
    setActiveTab(tab)
    setResults(null)
    // Update URL hash to match active tab
    if (typeof window !== 'undefined') {
      window.location.hash = tab;
    }
  }

  const handleOperationResult = (result) => {
    setResults(result)
  }

  const handleSchemaUpdated = () => {
    refetchSchemas()
  }

  const renderActiveTab = () => {
    switch (activeTab) {
      case 'schemas':
        return (
          <SchemaTab
            onResult={handleOperationResult}
            onSchemaUpdated={handleSchemaUpdated}
          />
        )
      case 'query':
        return <QueryTab onResult={handleOperationResult} />
      case 'llm-query':
        return <LlmQueryTab onResult={handleOperationResult} />
      case 'mutation':
        return <MutationTab onResult={handleOperationResult} />
      case 'smart-folder':
        return <SmartFolderTab onResult={handleOperationResult} />
      case 'ingestion':
        return <IngestionTab onResult={handleOperationResult} />
      case 'file-upload':
        return <FileUploadTab onResult={handleOperationResult} />
      case 'native-index':
        return <NativeIndexTab onResult={handleOperationResult} />
      case 'data-browser':
        return <DataBrowserTab />
      default:
        return null
    }
  }

  // Show login page if not authenticated
  if (!isAuthenticated && !isAuthLoading) {
    return <LoginPage />;
  }

  // Show loading spinner while restoring session or checking auth
  if (isAuthLoading) {
    return (
      <div className="h-screen flex items-center justify-center bg-surface-secondary">
        <div className="text-center">
          <div className="w-6 h-6 border-2 border-border border-t-primary rounded-full animate-spin mx-auto mb-4" />
          <p className="text-secondary text-sm">Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-surface overflow-hidden">
      <Header
        onSettingsClick={() => { setSettingsInitialTab(null); setIsSettingsOpen(true) }}
        onAiSettingsClick={() => { setSettingsInitialTab('ai'); setIsSettingsOpen(true) }}
        ingestionStatus={ingestionStatus}
      />
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} onConfigSaved={refetchIngestionStatus} initialTab={settingsInitialTab} />
      <OnboardingWizard isOpen={isOnboardingOpen} onClose={handleOnboardingClose} userHash={userHash} />

      {showSetupBanner && (
        <div className="bg-gruvbox-elevated border-b border-border px-8 py-3 flex items-center justify-between">
          <span className="text-gruvbox-blue text-sm">
            Configure AI to get started — FoldDB needs OpenRouter or local Ollama for ingestion and search.
          </span>
          <div className="flex items-center gap-3">
            <button
              onClick={() => setIsSettingsOpen(true)}
              className="bg-gruvbox-blue text-surface text-sm px-4 py-1.5 border-none cursor-pointer hover:bg-gruvbox-green transition-colors"
            >
              Configure AI
            </button>
            <button
              onClick={() => {
                setSetupDismissed(true)
                localStorage.setItem('folddb_setup_dismissed', '1')
              }}
              className="text-gruvbox-blue text-sm bg-transparent border-none cursor-pointer hover:text-gruvbox-bright transition-colors"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}

      <div className="flex flex-1 overflow-hidden">
        <div className="flex-1 flex flex-col overflow-hidden">
          <TabNavigation
            activeTab={activeTab}
            onTabChange={handleTabChange}
          />

          <main className="flex-1 overflow-y-auto bg-surface-secondary">
            <div className="max-w-5xl mx-auto p-6 bg-surface min-h-full">
              {/* Schema Loading/Error States */}
              {schemasError && (
                <div className="mb-4 p-3 bg-surface border border-border border-l-4 border-l-gruvbox-red">
                  <p className="text-gruvbox-red text-sm">{schemasError}</p>
                </div>
              )}

              {/* Section Title */}
              <div className="text-xs uppercase tracking-widest text-tertiary mb-3">
                {activeTab.replace('-', ' ')}
              </div>

              {/* Tab Content */}
              {renderActiveTab()}

              {/* Results */}
              {results && (
                <div className="mt-6">
                  <div className="text-xs uppercase tracking-widest text-tertiary mb-3">
                    Results
                  </div>
                  <ResultsSection results={results} />
                </div>
              )}
            </div>
          </main>
        </div>

        <LogSidebar />
      </div>

      <Footer />
    </div>
  )
}

function App() {
  return (
    <FoldDbProvider>
      <AppContent />
    </FoldDbProvider>
  )
}

export default App
